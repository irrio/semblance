use std::{
    ffi::{CStr, c_char},
    io::{Write, stdout},
    num::{ParseFloatError, ParseIntError},
    path::{Path, PathBuf},
};

use semblance::{
    inst::{WasmExternVal, WasmNumValue, WasmStore, WasmValue, table::WasmInstanceAddr},
    module::{WasmImportDesc, WasmMemIdx, WasmModule, WasmNumType, WasmValueType},
};

const HELP_TEXT: &'static str = "
semblance <MODULE> [OPTIONS]

Options:
    -h, --help                      Print this help text
    -I, --invoke <FN> [ARGS...]     Invoke an exported function
";

#[derive(Debug)]
struct CliArgs {
    pub module_path: PathBuf,
    pub invoke: Option<InvokeArgs>,
}

#[derive(Debug)]
struct InvokeArgs {
    pub fn_name: String,
    pub argv: Vec<String>,
}

enum CliFlag<'s> {
    Module(PathBuf),
    Invoke(Option<InvokeArgs>),
    Help,
    Noop,
    Unknown(&'s str),
}

fn parse_flag<'a>(argv: &'a [&'a str]) -> (CliFlag<'a>, &'a [&'a str]) {
    debug_assert!(argv.len() > 0);

    match argv {
        ["--", rest @ ..] => (CliFlag::Noop, rest),
        ["-h" | "--help", rest @ ..] => (CliFlag::Help, rest),
        ["-I" | "--invoke", rest @ ..] => {
            let (i, rest) = parse_invoke_args(rest);
            (CliFlag::Invoke(i), rest)
        }
        [s, rest @ ..] if s.starts_with("-") => (CliFlag::Unknown(s), rest),
        [s, rest @ ..] => (CliFlag::Module(PathBuf::from(s)), rest),
        _ => unreachable!("argv is not empty"),
    }
}

fn parse_invoke_args<'a>(argv: &'a [&'a str]) -> (Option<InvokeArgs>, &'a [&'a str]) {
    let idx = argv.iter().take_while(|s| !is_flag(s)).count();
    let inv = &argv[0..idx];
    let rest = &argv[idx..];
    if let Some((fn_name, args)) = inv.split_first() {
        (
            Some(InvokeArgs {
                fn_name: fn_name.to_string(),
                argv: args.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
            }),
            rest,
        )
    } else {
        (None, rest)
    }
}

fn is_flag(s: &str) -> bool {
    s.chars().nth(0) == Some('-')
        && if let Some(c) = s.chars().nth(1) {
            !c.is_digit(10)
        } else {
            false
        }
}

impl CliArgs {
    pub fn parse_or_exit() -> Self {
        fn exit() -> ! {
            std::process::exit(1)
        }

        let mut help = false;
        let mut module_path = None;
        let mut invoke = None;

        let argv = std::env::args().collect::<Vec<_>>();
        let strs = argv.iter().map(|s| s.as_str()).collect::<Vec<_>>();
        let mut rem = &strs[1..];

        while rem.len() > 0 {
            let (flag, rest) = parse_flag(rem);
            match flag {
                CliFlag::Noop => {}
                CliFlag::Help => help = true,
                CliFlag::Invoke(i) => {
                    if let Some(i) = i {
                        invoke = Some(i)
                    } else {
                        eprintln!("--invoke missing <FN>");
                        exit();
                    }
                }
                CliFlag::Module(m) => module_path = Some(m),
                CliFlag::Unknown(f) => {
                    eprintln!("unknown flag: {}", f);
                    exit();
                }
            }
            rem = rest;
        }

        if help {
            eprintln!("{}", HELP_TEXT);
            exit();
        }

        if let Some(module_path) = module_path {
            CliArgs {
                module_path,
                invoke,
            }
        } else {
            eprintln!("<MODULE> is required");
            exit();
        }
    }
}

fn read_module_or_exit(path: &Path) -> WasmModule {
    match WasmModule::read(path) {
        Ok(module) => module,
        Err(e) => {
            eprintln!("failed to load module: {:?}", e);
            std::process::exit(3);
        }
    }
}

#[derive(Debug)]
enum ParseArgError {
    Int(ParseIntError),
    Float(ParseFloatError),
}

fn parse_arg_with_type(ty: &WasmValueType, argv: &str) -> Result<WasmValue, ParseArgError> {
    let parsed = match ty {
        WasmValueType::Num(numt) => match numt {
            WasmNumType::I32 => WasmValue {
                num: WasmNumValue {
                    i32: argv.parse().map_err(ParseArgError::Int)?,
                },
            },
            WasmNumType::I64 => WasmValue {
                num: WasmNumValue {
                    i64: argv.parse().map_err(ParseArgError::Int)?,
                },
            },
            WasmNumType::F32 => WasmValue {
                num: WasmNumValue {
                    f32: argv.parse().map_err(ParseArgError::Float)?,
                },
            },
            WasmNumType::F64 => WasmValue {
                num: WasmNumValue {
                    f64: argv.parse().map_err(ParseArgError::Float)?,
                },
            },
        },
        WasmValueType::Vec(vect) => todo!(),
        WasmValueType::Ref(reft) => todo!(),
    };
    Ok(parsed)
}

fn parse_args_for_input_type(
    ty: &[WasmValueType],
    argv: &[String],
) -> Result<Box<[WasmValue]>, ParseArgError> {
    assert_eq!(ty.len(), argv.len());
    let mut parsed = Vec::with_capacity(ty.len());
    for (ty, argv) in ty.iter().zip(argv) {
        parsed.push(parse_arg_with_type(ty, argv)?);
    }
    Ok(parsed.into_boxed_slice())
}

#[derive(Debug)]
enum LinkError<'wmod> {
    UnknownSymbol(&'wmod str, &'wmod str),
}

fn hostcall_puts(
    store: &mut WasmStore,
    winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let offset = unsafe { args.get_unchecked(0).num.i32 };
    let memaddr = store.instances.resolve(winst_id).addr_of(WasmMemIdx::ZERO);
    let mem = store.mems.resolve(memaddr);
    let ptr = (&mem.data[offset as usize..]).as_ptr().cast::<c_char>();
    let cstr = unsafe { CStr::from_ptr(ptr) };
    let str = cstr.to_str().expect("invalid utf8");
    println!("{}", str);
    Box::new([])
}

fn resolve_imports<'wmod>(
    store: &mut WasmStore<'wmod>,
    wmod: &'wmod WasmModule,
) -> Result<Vec<WasmExternVal>, LinkError<'wmod>> {
    let mut externvals = Vec::with_capacity(wmod.imports.len());
    for import in &wmod.imports {
        if let WasmImportDesc::Func(typeidx) = import.desc {
            let ty = &wmod.types[typeidx.0 as usize];
            if import.module_name.0.as_ref() == "env" && import.item_name.0.as_ref() == "puts" {
                let funcaddr = store.alloc_hostfunc(ty, &hostcall_puts);
                externvals.push(WasmExternVal::Func(funcaddr));
                continue;
            }
        }
        return Err(LinkError::UnknownSymbol(
            &import.module_name.0,
            &import.item_name.0,
        ));
    }
    Ok(externvals)
}

fn main() {
    let args = CliArgs::parse_or_exit();
    let module = read_module_or_exit(&args.module_path);

    if let Some(InvokeArgs { fn_name, argv }) = args.invoke {
        let mut store = WasmStore::new();
        let externvals = resolve_imports(&mut store, &module).expect("link error");
        let winst_id = store
            .instantiate(&module, &externvals)
            .expect("failed to instantiate");
        let funcaddr = store
            .instances
            .resolve(winst_id)
            .exports
            .iter()
            .find_map(|exp| {
                if exp.name == fn_name {
                    if let WasmExternVal::Func(funcaddr) = exp.value {
                        return Some(funcaddr);
                    }
                }
                None
            })
            .expect("no function export found with name");
        let ty = store.funcs.resolve(funcaddr).type_.input_type.0.as_ref();
        let args = parse_args_for_input_type(ty, &argv).expect("failed to parse args");
        let wres = store.invoke(funcaddr, args).expect("trap!");
        println!("{}", wres);
    }
}
