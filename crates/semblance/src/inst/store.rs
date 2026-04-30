use std::ops::Deref;

use crate::{
    exec::exec,
    inst::{
        DynamicWasmResult, WasmFrame, WasmFuncAddr, WasmGlobalAddr, WasmInstanceAddr, WasmLabel,
        WasmMemAddr, WasmModuleInst, WasmRefValue, WasmResult, WasmStack, WasmTableAddr, WasmTrap,
        WasmValue, hostfunc::WasmHostFunc,
    },
    module::{
        WasmFunc, WasmFuncType, WasmGlobalType, WasmInstruction, WasmMemType, WasmRefType,
        WasmTableType,
    },
};

use super::table::StoreTable;

pub struct WasmStore {
    pub instances: StoreTable<WasmModuleInst>,
    pub funcs: StoreTable<WasmFuncInst>,
    pub tables: StoreTable<WasmTableInst>,
    pub mems: StoreTable<WasmMemInst>,
    pub globals: StoreTable<WasmGlobalInst>,
    pub elems: StoreTable<WasmElemInst>,
    pub datas: StoreTable<WasmDataInst>,
}

impl WasmStore {
    pub fn new() -> Self {
        WasmStore {
            instances: StoreTable::new(),
            funcs: StoreTable::new(),
            tables: StoreTable::new(),
            mems: StoreTable::new(),
            globals: StoreTable::new(),
            elems: StoreTable::new(),
            datas: StoreTable::new(),
        }
    }

    pub fn invoke(
        &mut self,
        funcaddr: WasmFuncAddr,
        args: Box<[WasmValue]>,
        opts: WasmInvokeOptions,
    ) -> Result<DynamicWasmResult, WasmTrap> {
        let func = self.funcs.resolve(funcaddr);
        let ty = func.type_;
        match func.impl_ {
            WasmFuncImpl::Wasm { winst_id, func } => {
                let mut stack = WasmStack::new(opts.max_control_stack_depth);
                let mut locals = args.into_vec();
                // todo: typecheck args
                for local_type in &func.locals {
                    locals.push(WasmValue::default_of_type(local_type));
                }
                stack.push_frame(WasmFrame {
                    locals: locals.into_boxed_slice(),
                    winst_id,
                })?;
                stack.push_label(WasmLabel {
                    instr: func.body.last().expect("func body has no end instr"),
                })?;
                exec(&mut stack, self, &func.body)?;
                let mut out = Vec::with_capacity(ty.output_type.0.len());
                for _ in 0..ty.output_type.0.len() {
                    out.push(stack.pop_value());
                }
                out.reverse();
                Ok(DynamicWasmResult {
                    ty: ty.output_type.0.clone(),
                    res: WasmResult(out),
                })
            }
            WasmFuncImpl::Host { hostfunc: _ } => todo!(),
        }
    }

    pub fn alloc_hostfunc(
        &mut self,
        type_: &'static WasmFuncType,
        hostfunc: WasmHostFunc,
    ) -> WasmFuncAddr {
        self.funcs.add(WasmFuncInst {
            type_: ModuleRef(type_),
            impl_: WasmFuncImpl::Host { hostfunc },
        })
    }

    pub fn alloc_host_global(
        &mut self,
        ty: &'static WasmGlobalType,
        wval: WasmValue,
    ) -> WasmGlobalAddr {
        self.globals.add(WasmGlobalInst {
            type_: ModuleRef(ty),
            val: wval,
        })
    }

    pub fn alloc_host_table(
        &mut self,
        ty: &'static WasmTableType,
        elems: Vec<WasmRefValue>,
    ) -> WasmTableAddr {
        self.tables.add(WasmTableInst {
            type_: ModuleRef(ty),
            elems,
        })
    }

    pub fn alloc_host_mem(&mut self, ty: &'static WasmMemType, bytes: Vec<u8>) -> WasmMemAddr {
        self.mems.add(WasmMemInst {
            type_: ModuleRef(ty),
            data: bytes,
        })
    }
}

pub struct ModuleRef<T: ?Sized>(pub *const T);

impl<T: ?Sized> Deref for ModuleRef<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { &*self.0 }
    }
}

impl<T: ?Sized> Clone for ModuleRef<T> {
    fn clone(&self) -> Self {
        ModuleRef(self.0)
    }
}

impl<T: ?Sized> Copy for ModuleRef<T> {}

#[derive(Debug)]
pub struct WasmInvokeOptions {
    max_control_stack_depth: usize,
}

impl Default for WasmInvokeOptions {
    fn default() -> Self {
        Self {
            max_control_stack_depth: 1024,
        }
    }
}

pub struct WasmDataInst {
    pub data: Option<ModuleRef<[u8]>>,
}

pub struct WasmElemInst {
    pub type_: WasmRefType,
    pub elem: Box<[WasmRefValue]>,
}

pub struct WasmGlobalInst {
    pub type_: ModuleRef<WasmGlobalType>,
    pub val: WasmValue,
}

pub struct WasmMemInst {
    pub type_: ModuleRef<WasmMemType>,
    pub data: Vec<u8>,
}

impl WasmMemInst {
    pub const PAGE_SIZE: usize = 65536;
}

pub struct WasmTableInst {
    pub type_: ModuleRef<WasmTableType>,
    pub elems: Vec<WasmRefValue>,
}

pub struct WasmFuncInst {
    pub type_: ModuleRef<WasmFuncType>,
    pub impl_: WasmFuncImpl,
}

pub enum WasmFuncImpl {
    Wasm {
        winst_id: WasmInstanceAddr,
        func: ModuleRef<WasmFunc<WasmInstruction>>,
    },
    Host {
        hostfunc: WasmHostFunc,
    },
}
