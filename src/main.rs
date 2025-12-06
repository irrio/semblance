use std::{
    ffi::{CStr, c_char},
    num::{ParseFloatError, ParseIntError},
    path::PathBuf,
};

use semblance::{
    inst::{
        DynamicWasmResult, WasmExternVal, WasmNumValue, WasmResult, WasmStore, WasmTrap, WasmValue,
        instantiate::WasmInstantiationError, table::WasmInstanceAddr,
    },
    module::{WasmImportDesc, WasmMemIdx, WasmModule, WasmNumType, WasmReadError, WasmValueType},
};

const HELP_TEXT: &'static str = "
semblance <MODULE> [OPTIONS]

Options:
    -h, --help                      Print this help text
    -I, --invoke <FN> [ARGS...]     Invoke an exported function
    --assert-return [VALUES...]     Assert that the invoked function returns a specific set of values
    --assert-trap [MESSAGE]         Assert that the invoked function traps
    --assert-invalid                Assert that the module does not pass validation
    --assert-malformed              Assert that the module cannot be decoded
";

#[derive(Debug)]
struct CliArgs {
    pub module_path: PathBuf,
    pub invoke: Option<InvokeArgs>,
    pub assert: Option<Assertion>,
}

#[derive(Debug)]
enum Assertion {
    Return(Vec<String>),
    Trap(Option<String>),
    Invalid,
    Malformed,
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
    AssertReturn(Vec<String>),
    AssertTrap(Option<String>),
    AssertInvalid,
    AssertMalformed,
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
        ["--assert-return", rest @ ..] => {
            let (vals, rest) = parse_assert_return_vals(rest);
            (CliFlag::AssertReturn(vals), rest)
        }
        ["--assert-trap", rest @ ..] => {
            let (message, rest) = parse_assert_trap_message(rest);
            (CliFlag::AssertTrap(message), rest)
        }
        ["--assert-invalid", rest @ ..] => (CliFlag::AssertInvalid, rest),
        ["--assert-malformed", rest @ ..] => (CliFlag::AssertMalformed, rest),
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

fn parse_assert_return_vals<'a>(argv: &'a [&'a str]) -> (Vec<String>, &'a [&'a str]) {
    let idx = argv.iter().take_while(|s| !is_flag(s)).count();
    let vals = argv[0..idx]
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let rest = &argv[idx..];
    (vals, rest)
}

fn parse_assert_trap_message<'a>(argv: &'a [&'a str]) -> (Option<String>, &'a [&'a str]) {
    if let Some(s) = argv.get(0)
        && is_flag(s)
    {
        (Some(s.to_string()), &argv[1..])
    } else {
        (None, argv)
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
        let mut assert = None;

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
                CliFlag::AssertReturn(vals) => assert = Some(Assertion::Return(vals)),
                CliFlag::AssertTrap(message) => assert = Some(Assertion::Trap(message)),
                CliFlag::AssertMalformed => assert = Some(Assertion::Malformed),
                CliFlag::AssertInvalid => assert = Some(Assertion::Invalid),
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
                assert,
            }
        } else {
            eprintln!("<MODULE> is required");
            exit();
        }
    }
}

#[derive(Debug)]
#[allow(dead_code)]
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
        WasmValueType::Vec(_vect) => todo!(),
        WasmValueType::Ref(_reft) => todo!(),
    };
    Ok(parsed)
}

fn parse_args_for_value_type(
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
#[allow(dead_code)]
enum WasmLinkError {
    UnknownSymbol(String, String),
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
    let str = cstr.to_string_lossy();
    println!("{}", str);
    Box::new([])
}

fn resolve_imports<'wmod>(
    store: &mut WasmStore<'wmod>,
    wmod: &'wmod WasmModule,
) -> Result<Vec<WasmExternVal>, WasmLinkError> {
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
        return Err(WasmLinkError::UnknownSymbol(
            import.module_name.0.to_string(),
            import.item_name.0.to_string(),
        ));
    }
    Ok(externvals)
}

#[derive(Debug)]
#[allow(dead_code)]
enum SemblanceError {
    Read(WasmReadError),
    Link(WasmLinkError),
    Instantiate(WasmInstantiationError),
    Args(ArgumentError),
    Trap(WasmTrap),
}

#[derive(Debug)]
#[allow(dead_code)]
enum ArgumentError {
    ExportNotFound(String),
    InvalidInput(ParseArgError),
}

type SemblanceResult = Result<DynamicWasmResult, SemblanceError>;

fn run(args: &CliArgs) -> SemblanceResult {
    let module = WasmModule::read(&args.module_path).map_err(SemblanceError::Read)?;
    if let Some(InvokeArgs {
        ref fn_name,
        ref argv,
    }) = args.invoke
    {
        let mut store = WasmStore::new();
        let externvals = resolve_imports(&mut store, &module).map_err(SemblanceError::Link)?;
        let winst_id = store
            .instantiate(&module, &externvals)
            .map_err(SemblanceError::Instantiate)?;
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
            .ok_or_else(|| {
                SemblanceError::Args(ArgumentError::ExportNotFound(fn_name.to_string()))
            })?;
        let ty = store.funcs.resolve(funcaddr).type_.input_type.0.as_ref();
        let invoke_args = parse_args_for_value_type(ty, &argv)
            .map_err(|e| SemblanceError::Args(ArgumentError::InvalidInput(e)))?;
        let wres = store
            .invoke(funcaddr, invoke_args)
            .map_err(SemblanceError::Trap)?;
        return Ok(wres);
    }
    Ok(DynamicWasmResult::void())
}

fn main() {
    let args = CliArgs::parse_or_exit();
    let wres = run(&args);

    if let Some(ref assertion) = args.assert {
        return apply_assertion(wres, &assertion);
    }

    match wres {
        Ok(v) => {
            if !v.ty.is_empty() {
                println!("{}", v);
            }
        }
        Err(e) => {
            eprintln!("{:?}", e);
            std::process::exit(1);
        }
    }
}

fn apply_assertion(wres: SemblanceResult, assertion: &Assertion) {
    match assertion {
        Assertion::Invalid => assert_invalid(wres),
        Assertion::Malformed => assert_malformed(wres),
        Assertion::Return(vals) => assert_return(wres, vals),
        Assertion::Trap(message) => assert_trap(wres, message),
    }
}

fn assert_invalid(wres: SemblanceResult) {
    match wres {
        Err(SemblanceError::Read(WasmReadError::Validation(_))) => {
            eprintln!("--assert-invalid passed");
        }
        _ => {
            eprintln!("--assert-invalid failed");
            eprintln!("expected an invalid module, got: ${:?}", wres);
            std::process::exit(1);
        }
    }
}

fn assert_malformed(wres: SemblanceResult) {
    match wres {
        Err(SemblanceError::Read(WasmReadError::Decode(_))) => {
            eprintln!("--assert-malformed passed");
        }
        _ => {
            eprintln!("--assert-malformed failed");
            eprintln!("expected a malformed module, got: ${:?}", wres);
            std::process::exit(1);
        }
    }
}

fn assert_return(wres: SemblanceResult, vals: &[String]) {
    match wres {
        Ok(v) => {
            let assert_vals =
                parse_args_for_value_type(&v.ty, vals).expect("failed to parse assert vals");
            let assert_dyn = DynamicWasmResult {
                ty: v.ty.clone(),
                res: WasmResult(assert_vals.into_vec()),
            };
            if assert_dyn == v {
                eprintln!("--assert-return passed");
            } else {
                eprintln!("--assert-return failed",);
                eprintln!("expected {} but got {}", assert_dyn, v);
                std::process::exit(1);
            }
        }
        _ => {
            eprintln!("--assert-return failed");
            eprintln!("expected return value but got {:?}", wres);
            std::process::exit(1);
        }
    }
}

fn assert_trap(wres: SemblanceResult, _message: &Option<String>) {
    match wres {
        Err(SemblanceError::Trap(_t)) => {
            eprintln!("--assert-trap passed");
        }
        _ => {
            eprintln!("--assert-trap failed");
            eprintln!("expected trap but got {:?}", wres);
            std::process::exit(1);
        }
    }
}
