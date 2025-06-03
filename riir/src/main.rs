use std::path::PathBuf;

use semblance::module::WasmModule;

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

fn main() {
    let args = CliArgs::parse_or_exit();
    println!("{:?}", args);

    let module = WasmModule::read(&args.module_path).expect("failed to load module");
    println!("{:?}", module);

    if let Some(InvokeArgs { fn_name, argv }) = args.invoke {
        eprintln!(
            "Invoking {} with args: {:?} not implemented yet!",
            fn_name, argv
        );
        std::process::exit(2);
    }
}
