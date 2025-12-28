use semblance::inst::table::WasmInstanceAddr;
use semblance::inst::{
    DynamicWasmResult, WasmNumValue, WasmResult, WasmStore, WasmTrap, WasmValue,
};
use semblance::link::WasmLinker;
use semblance::module::{WasmFromBytesError, WasmModule, WasmNumType, WasmValueType};
use std::io::Read;
use std::path::PathBuf;
use std::rc::Rc;
use wast::core::NanPattern;
use wast::parser::{ParseBuffer, parse};
use wast::token::{F32, F64, Id};
use wast::{QuoteWat, Wast, WastArg, WastDirective, WastExecute, WastInvoke, WastRet, Wat};

#[derive(Debug)]
struct WastArgs {
    wast_path: Option<PathBuf>,
}

fn parse_args(argv: &[String]) -> WastArgs {
    if argv.len() < 2 {
        WastArgs { wast_path: None }
    } else {
        WastArgs {
            wast_path: Some(PathBuf::from(&argv[1])),
        }
    }
}

fn read_wast(path: &Option<PathBuf>) -> std::io::Result<String> {
    let mut src = String::new();
    if let Some(path) = path {
        let mut f = std::fs::File::open(path)?;
        f.read_to_string(&mut src)?;
    } else {
        std::io::stdin().read_to_string(&mut src)?;
    }
    Ok(src)
}

struct WastInterpreter {
    linker: WasmLinker,
    current_inst: Option<(WasmStore, WasmInstanceAddr)>,
}

impl WastInterpreter {
    fn new() -> Self {
        WastInterpreter {
            linker: WasmLinker::new(),
            current_inst: None,
        }
    }

    fn eval_wast(&mut self, wast: &mut Wast) {
        let len = wast.directives.len();
        for (i, directive) in wast.directives.iter_mut().enumerate() {
            println!("running directive [{}/{}]", i, len);
            self.eval_directive(directive);
        }
    }

    fn eval_directive(&mut self, directive: &mut WastDirective) {
        use wast::WastDirective::*;
        match directive {
            Module(quote_wat) => {
                self.eval_module(quote_wat);
            }
            ModuleDefinition(quote_wat) => {
                todo!("module definition");
            }
            ModuleInstance {
                span,
                instance,
                module,
            } => {
                todo!("module instance");
            }
            AssertMalformed {
                span: _,
                module,
                message,
            } => {
                self.eval_assert_malformed(module, message);
            }
            AssertInvalid {
                span: _,
                module,
                message,
            } => {
                self.eval_assert_invalid(module, message);
            }
            Register { span, name, module } => {
                todo!("register")
            }
            Invoke(wast_invoke) => {
                let _ret = self.eval_invoke(wast_invoke).expect("trap!");
            }
            AssertTrap {
                span,
                exec,
                message,
            } => {
                self.eval_assert_trap(exec, message);
            }
            AssertReturn {
                span: _,
                exec,
                results,
            } => {
                self.eval_assert_return(exec, results);
            }
            AssertExhaustion {
                span,
                call,
                message,
            } => {
                todo!("assert exhaustion")
            }
            AssertUnlinkable {
                span,
                module,
                message,
            } => {
                todo!("assert unlinkable")
            }
            AssertException { span, exec } => {
                todo!("assert exception")
            }
            AssertSuspension {
                span,
                exec,
                message,
            } => {
                todo!("assert suspension")
            }
            Thread(wast_thread) => {
                todo!("thread")
            }
            Wait { span, thread } => {
                todo!("wait")
            }
        }
    }

    fn eval_module(&mut self, quote_wat: &mut QuoteWat) {
        let wmod = self
            .eval_quote_wat(quote_wat)
            .expect("failed to load module");
        let (mut store, externvals) = self.linker.link(&wmod).expect("failed to link module");
        let winst_id = store
            .instantiate(Rc::new(wmod), &externvals)
            .expect("failed to instantiate module");
        self.current_inst = Some((store, winst_id));
    }

    fn eval_assert_invalid(&mut self, module: &mut QuoteWat, _message: &str) {
        let res = self.eval_quote_wat(module);
        if let Err(WasmFromBytesError::Validation(_)) = res {
            // ok
        } else {
            panic!("expected invalid module, got: {:?}", res);
        }
    }

    fn eval_assert_malformed(&mut self, _module: &mut QuoteWat, _message: &str) {
        println!("skipping assert malformed");
        //let res = self.eval_quote_wat(module);
        //if let Err(WasmFromBytesError::Decode(_)) = res {
        //    // ok
        //} else {
        //    panic!("expected malformed module, got: {:?}", res);
        //}
    }

    fn eval_quote_wat(&mut self, module: &mut QuoteWat) -> Result<WasmModule, WasmFromBytesError> {
        let bytes = module.encode().expect("failed to encode wat");
        WasmModule::from_bytes(&bytes)
    }

    fn eval_assert_trap(&mut self, exec: &mut WastExecute, message: &str) {
        let wres = self.eval_execute(exec);
        assert!(wres.is_err(), "failed to trap! {}", message);
    }

    fn eval_assert_return(&mut self, exec: &mut WastExecute, results: &mut Vec<WastRet>) {
        let wres = self.eval_execute(exec).expect("trap!");
        self.assert_results(&wres, results);
    }

    fn assert_results(&mut self, wres: &DynamicWasmResult, results: &Vec<WastRet>) {
        assert!(wres.res.0.len() == results.len());
        for ((ty, val), wast_ret) in wres.ty.iter().zip(wres.res.0.iter()).zip(results) {
            self.assert_value(val, ty, wast_ret);
        }
    }

    fn assert_value(&mut self, val: &WasmValue, ty: &WasmValueType, wast_ret: &WastRet) {
        if let WastRet::Core(wast_ret) = wast_ret {
            match wast_ret {
                wast::core::WastRetCore::I32(i) => {
                    assert_eq!(*ty, WasmValueType::Num(WasmNumType::I32));
                    assert_eq!(unsafe { val.num.i32 }, *i);
                }
                wast::core::WastRetCore::I64(i) => {
                    assert_eq!(*ty, WasmValueType::Num(WasmNumType::I64));
                    assert_eq!(unsafe { val.num.i64 }, *i);
                }
                wast::core::WastRetCore::F32(nan_pattern) => {
                    assert_eq!(*ty, WasmValueType::Num(WasmNumType::F32));
                    assert_nan_pattern_32(nan_pattern, unsafe { val.num.f32 });
                }
                wast::core::WastRetCore::F64(nan_pattern) => {
                    assert_eq!(*ty, WasmValueType::Num(WasmNumType::F64));
                    assert_nan_pattern_64(nan_pattern, unsafe { val.num.f64 });
                }
                wast::core::WastRetCore::V128(v128_pattern) => todo!(),
                wast::core::WastRetCore::RefNull(heap_type) => todo!(),
                wast::core::WastRetCore::RefExtern(_) => todo!(),
                wast::core::WastRetCore::RefHost(_) => todo!(),
                wast::core::WastRetCore::RefFunc(index) => todo!(),
                wast::core::WastRetCore::RefAny => todo!(),
                wast::core::WastRetCore::RefEq => todo!(),
                wast::core::WastRetCore::RefArray => todo!(),
                wast::core::WastRetCore::RefStruct => todo!(),
                wast::core::WastRetCore::RefI31 => todo!(),
                wast::core::WastRetCore::RefI31Shared => todo!(),
                wast::core::WastRetCore::Either(wast_ret_cores) => todo!(),
            }
        } else {
            panic!("component model");
        }
    }

    fn eval_execute(&mut self, exec: &mut WastExecute) -> Result<DynamicWasmResult, WasmTrap> {
        match exec {
            WastExecute::Invoke(wast_invoke) => self.eval_invoke(wast_invoke),
            WastExecute::Wat(wat) => self.eval_exec_wat(wat),
            WastExecute::Get {
                span: _,
                module,
                global,
            } => self.eval_get(module.as_ref(), global),
        }
    }

    fn eval_invoke(&mut self, wast_invoke: &WastInvoke) -> Result<DynamicWasmResult, WasmTrap> {
        let args = self.eval_args(&wast_invoke.args);
        let (store, winst_id) = self.current_inst.as_mut().expect("no inst!");
        let winst = store.instances.resolve(*winst_id);
        let funcaddr = winst
            .resolve_export_fn_by_name(wast_invoke.name)
            .expect("fn not found");
        store.invoke(funcaddr, args)
    }

    fn eval_args(&self, args: &[WastArg]) -> Box<[WasmValue]> {
        let mut args_out = Vec::with_capacity(args.len());
        for arg in args {
            args_out.push(self.eval_arg(arg));
        }
        args_out.into_boxed_slice()
    }

    fn eval_arg(&self, arg: &WastArg) -> WasmValue {
        if let WastArg::Core(arg) = arg {
            match arg {
                wast::core::WastArgCore::I32(i) => WasmValue {
                    num: WasmNumValue { i32: *i },
                },
                wast::core::WastArgCore::I64(i) => WasmValue {
                    num: WasmNumValue { i64: *i },
                },
                wast::core::WastArgCore::F32(f) => WasmValue {
                    num: WasmNumValue {
                        f32: f32::from_bits(f.bits),
                    },
                },
                wast::core::WastArgCore::F64(f) => WasmValue {
                    num: WasmNumValue {
                        f64: f64::from_bits(f.bits),
                    },
                },
                wast::core::WastArgCore::V128(_v) => todo!("vec arg"),
                wast::core::WastArgCore::RefNull(_heap_type) => todo!("ref null arg"),
                wast::core::WastArgCore::RefExtern(_) => todo!("externref arg"),
                wast::core::WastArgCore::RefHost(_) => todo!("hostref arg"),
            }
        } else {
            panic!("unsupported arg");
        }
    }

    fn eval_exec_wat(&mut self, wat: &Wat) -> Result<DynamicWasmResult, WasmTrap> {
        todo!("exec wat");
    }

    fn eval_get(
        &mut self,
        module: Option<&Id>,
        global_name: &str,
    ) -> Result<DynamicWasmResult, WasmTrap> {
        if module.is_some() {
            todo!("eval get with module id")
        }
        let (store, winst_id) = self.current_inst.as_mut().expect("no inst!");
        let winst = store.instances.resolve(*winst_id);
        let globaladdr = winst
            .resolve_export_global_by_name(global_name)
            .expect("global not found");
        let global = store.globals.resolve(globaladdr);
        Ok(DynamicWasmResult {
            ty: Box::new([global.type_.val_type]),
            res: WasmResult(vec![global.val]),
        })
    }
}

fn assert_nan_pattern_32(nan_pattern: &NanPattern<F32>, val: f32) {
    match nan_pattern {
        NanPattern::ArithmeticNan => assert!(val.is_nan()),
        NanPattern::CanonicalNan => assert!(val.is_nan()),
        NanPattern::Value(f) => assert_eq!(f.bits, val.to_bits()),
    }
}

fn assert_nan_pattern_64(nan_pattern: &NanPattern<F64>, val: f64) {
    match nan_pattern {
        NanPattern::ArithmeticNan => assert!(val.is_nan()),
        NanPattern::CanonicalNan => assert!(val.is_nan()),
        NanPattern::Value(f) => assert_eq!(f.bits, val.to_bits()),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let argv = std::env::args().collect::<Vec<_>>();
    let args = parse_args(&argv);

    let src = read_wast(&args.wast_path)?;
    let buf = ParseBuffer::new(&src)?;
    let mut wast: Wast = parse(&buf)?;

    let mut interpreter = WastInterpreter::new();
    interpreter.eval_wast(&mut wast);
    Ok(())
}
