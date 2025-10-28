use std::fmt::Display;

use table::{
    StoreTable, WasmDataAddr, WasmElemAddr, WasmFuncAddr, WasmGlobalAddr, WasmInstanceAddr,
    WasmMemAddr, WasmTableAddr,
};

use crate::{exec::exec, module::*};

pub mod instantiate;
pub mod table;

pub struct WasmValueStack(Vec<WasmValue>);

impl WasmValueStack {
    pub fn new() -> Self {
        WasmValueStack(Vec::new())
    }

    pub fn push<I: Into<WasmValue>>(&mut self, val: I) {
        self.0.push(val.into())
    }

    pub fn pop(&mut self) -> WasmValue {
        self.0.pop().expect("value stack underflow")
    }
}

pub struct WasmStack {
    value_stack: WasmValueStack,
    label_stack: Vec<WasmLabel>,
    call_stack: Vec<WasmFrame>,
}

impl WasmStack {
    pub fn new() -> Self {
        WasmStack {
            value_stack: WasmValueStack::new(),
            label_stack: Vec::new(),
            call_stack: Vec::new(),
        }
    }

    pub fn push_value<V: Into<WasmValue>>(&mut self, val: V) {
        self.value_stack.push(val);
    }

    pub fn pop_value(&mut self) -> WasmValue {
        self.value_stack.pop()
    }

    pub fn pop_values(&mut self, n: usize) -> Vec<WasmValue> {
        let mut out = Vec::with_capacity(n);
        for _ in 0..n {
            out.push(self.pop_value());
        }
        out
    }

    pub fn push_label(&mut self, label: WasmLabel) {
        self.label_stack.push(label);
    }

    pub fn pop_label(&mut self, label_idx: WasmLabelIdx) -> Option<WasmLabel> {
        let mut n = label_idx.0 + 1;
        let mut label = None;
        while n > 0 {
            label = self.label_stack.pop();
            n -= 1;
        }
        label
    }

    pub fn push_frame(&mut self, frame: WasmFrame) {
        self.call_stack.push(frame);
    }

    pub fn pop_frame(&mut self) -> WasmFrame {
        self.call_stack.pop().expect("call stack underflow")
    }

    pub fn current_frame(&self) -> &WasmFrame {
        self.call_stack.last().expect("no call frame")
    }

    pub fn current_frame_mut(&mut self) -> &mut WasmFrame {
        self.call_stack.last_mut().expect("no call frame")
    }
}

pub struct WasmFrame {
    pub arity: u32,
    pub locals: Box<[WasmValue]>,
    pub winst_id: WasmInstanceAddr,
}

pub struct WasmLabel {
    pub arity: u32,
    pub instr: WasmInstructionIdx,
}

pub struct WasmModuleInst<'wmod> {
    pub types: &'wmod [WasmFuncType],
    pub funcaddrs: Box<[WasmFuncAddr]>,
    pub tableaddrs: Box<[WasmTableAddr]>,
    pub memaddrs: Box<[WasmMemAddr]>,
    pub globaladdrs: Box<[WasmGlobalAddr]>,
    pub elemaddrs: Box<[WasmElemAddr]>,
    pub dataaddrs: Box<[WasmDataAddr]>,
    pub exports: Box<[WasmExportInst<'wmod>]>,
}

pub trait WasmIdx {
    type Addr;
    fn resolve_addr(self, winst: &WasmModuleInst) -> Self::Addr;
}

impl WasmIdx for WasmFuncIdx {
    type Addr = WasmFuncAddr;
    fn resolve_addr(self, winst: &WasmModuleInst) -> Self::Addr {
        unsafe { *winst.funcaddrs.get_unchecked(self.0 as usize) }
    }
}

impl WasmIdx for WasmTableIdx {
    type Addr = WasmTableAddr;
    fn resolve_addr(self, winst: &WasmModuleInst) -> Self::Addr {
        unsafe { *winst.tableaddrs.get_unchecked(self.0 as usize) }
    }
}

impl WasmIdx for WasmMemIdx {
    type Addr = WasmMemAddr;
    fn resolve_addr(self, winst: &WasmModuleInst) -> Self::Addr {
        unsafe { *winst.memaddrs.get_unchecked(self.0 as usize) }
    }
}

impl WasmIdx for WasmGlobalIdx {
    type Addr = WasmGlobalAddr;
    fn resolve_addr(self, winst: &WasmModuleInst) -> Self::Addr {
        unsafe { *winst.globaladdrs.get_unchecked(self.0 as usize) }
    }
}

impl WasmIdx for WasmElemIdx {
    type Addr = WasmElemAddr;
    fn resolve_addr(self, winst: &WasmModuleInst) -> Self::Addr {
        unsafe { *winst.elemaddrs.get_unchecked(self.0 as usize) }
    }
}

impl WasmIdx for WasmDataIdx {
    type Addr = WasmDataAddr;
    fn resolve_addr(self, winst: &WasmModuleInst) -> Self::Addr {
        unsafe { *winst.dataaddrs.get_unchecked(self.0 as usize) }
    }
}

impl<'wmod> WasmModuleInst<'wmod> {
    pub fn addr_of<I: WasmIdx>(&self, idx: I) -> I::Addr {
        idx.resolve_addr(self)
    }
}

pub struct WasmExportInst<'wmod> {
    pub name: &'wmod str,
    pub value: WasmExternVal,
}

#[derive(Debug)]
pub enum WasmExternVal {
    Func(WasmFuncAddr),
    Table(WasmTableAddr),
    Mem(WasmMemAddr),
    Global(WasmGlobalAddr),
}

pub struct WasmStore<'wmod> {
    pub instances: StoreTable<WasmModuleInst<'wmod>>,
    pub funcs: StoreTable<WasmFuncInst<'wmod>>,
    pub tables: StoreTable<WasmTableInst<'wmod>>,
    pub mems: StoreTable<WasmMemInst<'wmod>>,
    pub globals: StoreTable<WasmGlobalInst<'wmod>>,
    pub elems: StoreTable<WasmElemInst>,
    pub datas: StoreTable<WasmDataInst<'wmod>>,
}

impl<'wmod> WasmStore<'wmod> {
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
        mut args: Box<[WasmValue]>,
    ) -> Result<DynamicWasmResult<'wmod>, WasmTrap> {
        let func = self.funcs.resolve(funcaddr);
        args.reverse();
        match func.impl_ {
            WasmFuncImpl::Wasm { winst_id, func } => {
                let mut stack = WasmStack::new();
                // todo: typecheck args
                let ty = &self.instances.resolve(winst_id).types[func.type_idx.0 as usize];
                let arity = ty.output_type.0.len() as u32;
                stack.push_frame(WasmFrame {
                    arity,
                    locals: args,
                    winst_id,
                });
                stack.push_label(WasmLabel {
                    arity,
                    instr: WasmInstructionIdx(func.body.len() as u32 - 1),
                });
                exec(&mut stack, self, &func.body)?;
                let mut out = Vec::with_capacity(ty.output_type.0.len());
                for _ in 0..ty.output_type.0.len() {
                    out.push(stack.pop_value());
                }
                Ok(DynamicWasmResult {
                    ty: &ty.output_type.0,
                    res: WasmResult(out),
                })
            }
            WasmFuncImpl::Host { hostfunc: _ } => todo!(),
        }
    }

    pub fn alloc_hostfunc(
        &mut self,
        type_: &'wmod WasmFuncType,
        hostfunc: WasmHostFunc,
    ) -> WasmFuncAddr {
        self.funcs.add(WasmFuncInst {
            type_,
            impl_: WasmFuncImpl::Host { hostfunc },
        })
    }
}

pub struct WasmDataInst<'wmod> {
    pub data: Option<&'wmod [u8]>,
}

pub struct WasmElemInst {
    pub type_: WasmRefType,
    pub elem: Box<[WasmRefValue]>,
}

pub struct WasmGlobalInst<'wmod> {
    pub type_: &'wmod WasmGlobalType,
    pub val: WasmValue,
}

pub struct WasmMemInst<'wmod> {
    pub type_: &'wmod WasmMemType,
    pub data: Vec<u8>,
}

impl<'wmod> WasmMemInst<'wmod> {
    pub const PAGE_SIZE: usize = 65536;
}

pub struct WasmTableInst<'wmod> {
    pub type_: &'wmod WasmTableType,
    pub elems: Vec<WasmRefValue>,
}

pub struct WasmFuncInst<'wmod> {
    pub type_: &'wmod WasmFuncType,
    pub impl_: WasmFuncImpl<'wmod>,
}

pub enum WasmFuncImpl<'wmod> {
    Wasm {
        winst_id: WasmInstanceAddr,
        func: &'wmod WasmFunc<WasmInstruction>,
    },
    Host {
        hostfunc: WasmHostFunc,
    },
}

pub type WasmHostFunc =
    &'static dyn Fn(&mut WasmStore, WasmInstanceAddr, &[WasmValue]) -> Box<[WasmValue]>;

pub struct WasmResult(pub Vec<WasmValue>);

pub struct DynamicWasmResult<'wmod> {
    pub ty: &'wmod [WasmValueType],
    pub res: WasmResult,
}

impl<'wmod> Display for DynamicWasmResult<'wmod> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.ty.len() != 1 {
            write!(f, "(")?;
        }
        for (i, (ty, val)) in self.ty.iter().zip(self.res.0.iter()).enumerate() {
            match ty {
                WasmValueType::Num(numt) => match numt {
                    WasmNumType::I32 => write!(f, "{}", unsafe { val.num.i32 })?,
                    WasmNumType::I64 => write!(f, "{}", unsafe { val.num.i64 })?,
                    WasmNumType::F32 => write!(f, "{}", unsafe { val.num.f32 })?,
                    WasmNumType::F64 => write!(f, "{}", unsafe { val.num.f64 })?,
                },
                WasmValueType::Vec(vect) => {
                    todo!()
                }
                WasmValueType::Ref(reft) => {
                    todo!()
                }
            }
            if i < self.ty.len() - 1 {
                write!(f, ", ")?;
            }
        }
        if self.ty.len() != 1 {
            write!(f, ")")?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct WasmTrap {}

#[derive(Clone, Copy)]
pub union WasmValue {
    pub num: WasmNumValue,
    pub vec: WasmVecValue,
    pub ref_: WasmRefValue,
}

impl Into<WasmValue> for i32 {
    fn into(self) -> WasmValue {
        WasmValue {
            num: WasmNumValue { i32: self },
        }
    }
}

impl Into<WasmValue> for i64 {
    fn into(self) -> WasmValue {
        WasmValue {
            num: WasmNumValue { i64: self },
        }
    }
}

impl Into<WasmValue> for f32 {
    fn into(self) -> WasmValue {
        WasmValue {
            num: WasmNumValue { f32: self },
        }
    }
}

impl Into<WasmValue> for f64 {
    fn into(self) -> WasmValue {
        WasmValue {
            num: WasmNumValue { f64: self },
        }
    }
}

#[derive(Clone, Copy)]
pub union WasmNumValue {
    pub i32: i32,
    pub i64: i64,
    pub f32: f32,
    pub f64: f64,
}

pub type WasmVecValue = i128;

#[derive(Clone, Copy)]
pub union WasmRefValue {
    pub func: WasmFuncAddr,
    pub extern_: WasmExternAddr,
}

#[derive(Debug, Copy, Clone)]
pub struct WasmExternAddr(u32);

impl Into<WasmValue> for WasmRefValue {
    fn into(self) -> WasmValue {
        WasmValue { ref_: self }
    }
}

impl WasmRefValue {
    pub const NULL: WasmRefValue = WasmRefValue {
        func: WasmFuncAddr::NULL,
    };
}
