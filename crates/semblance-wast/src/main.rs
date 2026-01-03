use semblance::inst::instantiate::{WasmInstantiationError, WasmInstantiationResult};
use semblance::inst::table::WasmInstanceAddr;
use semblance::inst::{
    DynamicWasmResult, WasmExternAddr, WasmExternVal, WasmMemInst, WasmNumValue, WasmRefValue,
    WasmResult, WasmStore, WasmTrap, WasmValue,
};
use semblance::module::{
    WasmFromBytesError, WasmFuncType, WasmGlobalMutability, WasmGlobalType, WasmLimits,
    WasmMemType, WasmModule, WasmNumType, WasmRefType, WasmResultType, WasmTableType,
    WasmValueType,
};
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::LazyLock;
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
    store: WasmStore,
    registry: HashMap<String, WasmInstanceAddr>,
    linker_symbols: HashMap<String, WasmInstanceAddr>,
    spectest_exports: HashMap<&'static str, WasmExternVal>,
    current_inst: Option<WasmInstanceAddr>,
}

static HOSTCALL_PRINT_I32_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::I32)])),
    output_type: WasmResultType(Box::new([])),
});

fn hostcall_print_i32(
    _store: &mut WasmStore,
    _winst: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    println!("{}", unsafe { args[0].num.i32 });
    Box::new([])
}

static HOSTCALL_PRINT_I64_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::I64)])),
    output_type: WasmResultType(Box::new([])),
});

fn hostcall_print_i64(
    _store: &mut WasmStore,
    _winst: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    println!("{}", unsafe { args[0].num.i64 });
    Box::new([])
}

static HOSTCALL_PRINT_F32_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::F32)])),
    output_type: WasmResultType(Box::new([])),
});

fn hostcall_print_f32(
    _store: &mut WasmStore,
    _winst: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    println!("{}", unsafe { args[0].num.f32 });
    Box::new([])
}

static HOSTCALL_PRINT_F64_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::F64)])),
    output_type: WasmResultType(Box::new([])),
});

fn hostcall_print_f64(
    _store: &mut WasmStore,
    _winst: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    println!("{}", unsafe { args[0].num.f64 });
    Box::new([])
}

static HOSTCALL_PRINT_I32_F32_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32),
        WasmValueType::Num(WasmNumType::F32),
    ])),
    output_type: WasmResultType(Box::new([])),
});

fn hostcall_print_i32_f32(
    _store: &mut WasmStore,
    _winst: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    println!("{} {}", unsafe { args[0].num.i32 }, unsafe {
        args[1].num.f32
    });
    Box::new([])
}

static HOSTCALL_PRINT_F64_F64_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::F64),
        WasmValueType::Num(WasmNumType::F64),
    ])),
    output_type: WasmResultType(Box::new([])),
});

fn hostcall_print_f64_f64(
    _store: &mut WasmStore,
    _winst: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    println!("{} {}", unsafe { args[0].num.f64 }, unsafe {
        args[1].num.f64
    });
    Box::new([])
}

static HOST_GLOBAL_I32_TYPE: LazyLock<WasmGlobalType> = LazyLock::new(|| WasmGlobalType {
    mutability: WasmGlobalMutability::Immutable,
    val_type: WasmValueType::Num(WasmNumType::I32),
});

static HOST_GLOBAL_I64_TYPE: LazyLock<WasmGlobalType> = LazyLock::new(|| WasmGlobalType {
    mutability: WasmGlobalMutability::Immutable,
    val_type: WasmValueType::Num(WasmNumType::I64),
});

static HOST_GLOBAL_F32_TYPE: LazyLock<WasmGlobalType> = LazyLock::new(|| WasmGlobalType {
    mutability: WasmGlobalMutability::Immutable,
    val_type: WasmValueType::Num(WasmNumType::F32),
});

static HOST_GLOBAL_F64_TYPE: LazyLock<WasmGlobalType> = LazyLock::new(|| WasmGlobalType {
    mutability: WasmGlobalMutability::Immutable,
    val_type: WasmValueType::Num(WasmNumType::F64),
});

static HOST_TABLE_TYPE: LazyLock<WasmTableType> = LazyLock::new(|| WasmTableType {
    limits: WasmLimits {
        min: 10,
        max: Some(20),
    },
    ref_type: WasmRefType::FuncRef,
});

static HOST_MEM_TYPE: LazyLock<WasmMemType> = LazyLock::new(|| WasmMemType {
    limits: WasmLimits {
        min: 1,
        max: Some(2),
    },
});

impl WastInterpreter {
    fn new() -> Self {
        let mut store = WasmStore::new();
        let mut spectest_exports = HashMap::new();
        spectest_exports.insert(
            "print_i32",
            WasmExternVal::Func(
                store.alloc_hostfunc(&*HOSTCALL_PRINT_I32_TYPE, &hostcall_print_i32),
            ),
        );
        spectest_exports.insert(
            "print_i64",
            WasmExternVal::Func(
                store.alloc_hostfunc(&*HOSTCALL_PRINT_I64_TYPE, &hostcall_print_i64),
            ),
        );
        spectest_exports.insert(
            "print_f32",
            WasmExternVal::Func(
                store.alloc_hostfunc(&*HOSTCALL_PRINT_F32_TYPE, &hostcall_print_f32),
            ),
        );
        spectest_exports.insert(
            "print_f64",
            WasmExternVal::Func(
                store.alloc_hostfunc(&*HOSTCALL_PRINT_F64_TYPE, &hostcall_print_f64),
            ),
        );
        spectest_exports.insert(
            "print_i32_f32",
            WasmExternVal::Func(
                store.alloc_hostfunc(&*HOSTCALL_PRINT_I32_F32_TYPE, &hostcall_print_i32_f32),
            ),
        );
        spectest_exports.insert(
            "print_f64_f64",
            WasmExternVal::Func(
                store.alloc_hostfunc(&*HOSTCALL_PRINT_F64_F64_TYPE, &hostcall_print_f64_f64),
            ),
        );
        spectest_exports.insert(
            "global_i32",
            WasmExternVal::Global(store.alloc_host_global(
                &*HOST_GLOBAL_I32_TYPE,
                WasmValue {
                    num: { WasmNumValue { i32: 666 } },
                },
            )),
        );
        spectest_exports.insert(
            "global_i64",
            WasmExternVal::Global(store.alloc_host_global(
                &*HOST_GLOBAL_I64_TYPE,
                WasmValue {
                    num: { WasmNumValue { i64: 666 } },
                },
            )),
        );
        spectest_exports.insert(
            "global_f32",
            WasmExternVal::Global(store.alloc_host_global(
                &*HOST_GLOBAL_F32_TYPE,
                WasmValue {
                    num: { WasmNumValue { f32: 666.6 } },
                },
            )),
        );
        spectest_exports.insert(
            "global_f64",
            WasmExternVal::Global(store.alloc_host_global(
                &*HOST_GLOBAL_F64_TYPE,
                WasmValue {
                    num: { WasmNumValue { f64: 666.6 } },
                },
            )),
        );
        spectest_exports.insert(
            "table",
            WasmExternVal::Table(
                store.alloc_host_table(&*HOST_TABLE_TYPE, vec![WasmRefValue::NULL; 10]),
            ),
        );
        spectest_exports.insert(
            "memory",
            WasmExternVal::Mem(
                store.alloc_host_mem(&*HOST_MEM_TYPE, vec![0; WasmMemInst::PAGE_SIZE]),
            ),
        );
        WastInterpreter {
            store,
            spectest_exports,
            registry: HashMap::new(),
            linker_symbols: HashMap::new(),
            current_inst: None,
        }
    }

    fn eval_wast(&mut self, wast: &mut Wast, path: Option<&Path>, src: &str) {
        let len = wast.directives.len();
        let path_str = path
            .map(|p| p.to_string_lossy())
            .unwrap_or(std::borrow::Cow::Borrowed("stdin"));
        for (i, directive) in wast.directives.iter_mut().enumerate() {
            let (line, col) = directive.span().linecol_in(src);
            println!(
                "running directive [{}/{}] {}:{}:{}",
                i,
                len,
                path_str,
                line + 1,
                col,
            );
            self.eval_directive(directive);
        }
    }

    fn eval_directive(&mut self, directive: &mut WastDirective) {
        use wast::WastDirective::*;
        match directive {
            Module(quote_wat) => {
                self.eval_module(quote_wat);
            }
            ModuleDefinition(_quote_wat) => {
                todo!("module definition");
            }
            ModuleInstance {
                span: _,
                instance: _,
                module: _,
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
            Register {
                span: _,
                name,
                module,
            } => {
                let winst_id = if let Some(modname) = module {
                    *self
                        .registry
                        .get(modname.name())
                        .expect("no registered module!")
                } else {
                    self.current_inst.expect("no current inst!")
                };
                self.linker_symbols.insert(name.to_string(), winst_id);
            }
            Invoke(wast_invoke) => {
                let _ret = self.eval_invoke(wast_invoke).expect("trap!");
            }
            AssertTrap {
                span: _,
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
                span: _,
                call: _,
                message: _,
            } => {
                todo!("assert exhaustion")
            }
            AssertUnlinkable {
                span: _,
                module,
                message,
            } => {
                self.eval_assert_unlinkable(module, message);
            }
            AssertException { span: _, exec: _ } => {
                todo!("assert exception")
            }
            AssertSuspension {
                span: _,
                exec: _,
                message: _,
            } => {
                todo!("assert suspension")
            }
            Thread(_wast_thread) => {
                todo!("thread")
            }
            Wait { span: _, thread: _ } => {
                todo!("wait")
            }
        }
    }

    fn instantiate(&mut self, wmod: Rc<WasmModule>) -> WasmInstantiationResult<WasmInstanceAddr> {
        let mut externvals: Vec<WasmExternVal> = Vec::with_capacity(wmod.imports.len());
        for import in &wmod.imports {
            if import.module_name.0.as_ref() == "spectest" {
                let externval = self
                    .spectest_exports
                    .get(import.item_name.0.as_ref())
                    // TODO: make this error better, should be a link error
                    .ok_or(WasmInstantiationError::InvalidExternval)?;
                externvals.push(*externval);
            } else {
                let dep_inst_id = self
                    .linker_symbols
                    .get(import.module_name.0.as_ref())
                    .expect("unknown linker symbol");
                let dep_inst = self.store.instances.resolve(*dep_inst_id);
                let externval = dep_inst
                    .resolve_export_by_name(import.item_name.0.as_ref())
                    // TODO: make this error better, should be a link error
                    .ok_or(WasmInstantiationError::InvalidExternval)?;
                externvals.push(externval);
            }
        }
        self.store.instantiate(wmod, &externvals)
    }

    fn eval_module(&mut self, quote_wat: &mut QuoteWat) {
        let wmod = Rc::new(
            self.eval_quote_wat(quote_wat)
                .expect("failed to load module"),
        );
        let winst_id = self.instantiate(wmod).expect("failed to instantiate");
        if let Some(name) = quote_wat.name() {
            self.registry.insert(name.name().to_string(), winst_id);
        }
        self.current_inst = Some(winst_id);
    }

    fn eval_assert_invalid(&mut self, module: &mut QuoteWat, _message: &str) {
        let res = self.eval_quote_wat(module);
        if let Err(WasmFromBytesError::Validation(_)) = res {
            // ok
        } else {
            panic!("expected invalid module, got: {:?}", res);
        }
    }

    fn eval_assert_unlinkable(&mut self, module: &mut Wat, _message: &str) {
        let wmod = Rc::new(self.eval_wat(module).expect("failed to load module"));
        let res = self.instantiate(wmod);
        if let Err(_) = res {
            // ok
        } else {
            panic!("should be unlinkable");
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

    fn eval_wat(&mut self, module: &mut Wat) -> Result<WasmModule, WasmFromBytesError> {
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
                wast::core::WastRetCore::V128(_v128_pattern) => todo!(),
                wast::core::WastRetCore::RefNull(_heap_type) => {
                    assert!(ty.is_ref());
                    assert_eq!(0, unsafe { val.ref_.extern_.0 });
                }
                wast::core::WastRetCore::RefExtern(addr) => {
                    assert_eq!(*ty, WasmValueType::Ref(WasmRefType::ExternRef));
                    if let Some(addr) = addr {
                        assert_eq!(*addr, unsafe { val.ref_.extern_.0 });
                    }
                }
                wast::core::WastRetCore::RefHost(_) => todo!(),
                wast::core::WastRetCore::RefFunc(_index) => todo!(),
                wast::core::WastRetCore::RefAny => todo!(),
                wast::core::WastRetCore::RefEq => todo!(),
                wast::core::WastRetCore::RefArray => todo!(),
                wast::core::WastRetCore::RefStruct => todo!(),
                wast::core::WastRetCore::RefI31 => todo!(),
                wast::core::WastRetCore::RefI31Shared => todo!(),
                wast::core::WastRetCore::Either(_wast_ret_cores) => todo!(),
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
        let winst_id = if let Some(modname) = wast_invoke.module {
            *self
                .registry
                .get(modname.name())
                .expect("no inst with name")
        } else {
            self.current_inst.expect("no inst!")
        };
        let winst = self.store.instances.resolve(winst_id);
        let funcaddr = winst
            .resolve_export_fn_by_name(wast_invoke.name)
            .expect("fn not found");
        self.store.invoke(funcaddr, args)
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
                wast::core::WastArgCore::RefNull(_heap_type) => WasmValue {
                    ref_: WasmRefValue::NULL,
                },
                wast::core::WastArgCore::RefExtern(addr) => WasmValue {
                    ref_: WasmRefValue {
                        extern_: WasmExternAddr(*addr),
                    },
                },
                wast::core::WastArgCore::RefHost(_) => todo!("hostref arg"),
            }
        } else {
            panic!("unsupported arg");
        }
    }

    fn eval_exec_wat(&mut self, _wat: &Wat) -> Result<DynamicWasmResult, WasmTrap> {
        todo!("exec wat");
    }

    fn eval_get(
        &mut self,
        module: Option<&Id>,
        global_name: &str,
    ) -> Result<DynamicWasmResult, WasmTrap> {
        let winst_id = if let Some(modname) = module {
            *self
                .registry
                .get(modname.name())
                .expect("named module not found")
        } else {
            self.current_inst.expect("no inst!")
        };
        let winst = self.store.instances.resolve(winst_id);
        let globaladdr = winst
            .resolve_export_global_by_name(global_name)
            .expect("global not found");
        let global = self.store.globals.resolve(globaladdr);
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
    interpreter.eval_wast(
        &mut wast,
        args.wast_path.as_ref().map(|p| p.as_path()),
        &src,
    );
    Ok(())
}
