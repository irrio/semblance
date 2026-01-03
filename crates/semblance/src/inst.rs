use std::{fmt::Display, ops::Deref, rc::Rc};

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

pub struct WasmLabel {
    pub instr: *const WasmInstruction,
}

pub enum ControlStackEntry {
    Frame(WasmFrame),
    Label(WasmLabel),
}

pub struct WasmStack {
    value_stack: WasmValueStack,
    control_stack: Vec<ControlStackEntry>,
}

impl WasmStack {
    pub fn new() -> Self {
        WasmStack {
            value_stack: WasmValueStack::new(),
            control_stack: Vec::new(),
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
        out.reverse();
        out
    }

    pub fn push_label(&mut self, label: WasmLabel) {
        self.control_stack.push(ControlStackEntry::Label(label));
    }

    pub fn push_frame(&mut self, frame: WasmFrame) {
        self.control_stack.push(ControlStackEntry::Frame(frame));
    }

    pub fn pop_control(&mut self) -> Option<ControlStackEntry> {
        self.control_stack.pop()
    }

    pub fn pop_label(&mut self, label_idx: WasmLabelIdx) -> WasmLabel {
        let n = label_idx.0 + 1;
        self.control_stack
            .truncate(self.control_stack.len() - (n - 1) as usize);
        if let Some(ControlStackEntry::Label(label)) = self.control_stack.pop() {
            label
        } else {
            panic!("invalid labelidx");
        }
    }

    pub fn pop_frame(&mut self) -> WasmFrame {
        loop {
            match self.control_stack.pop() {
                Some(ControlStackEntry::Frame(frame)) => return frame,
                Some(ControlStackEntry::Label(_)) => continue,
                None => break,
            }
        }
        panic!("no call frame");
    }

    pub fn current_frame(&self) -> &WasmFrame {
        for entry in self.control_stack.iter().rev() {
            if let ControlStackEntry::Frame(frame) = entry {
                return frame;
            }
        }
        panic!("no call frame");
    }

    pub fn current_frame_mut(&mut self) -> &mut WasmFrame {
        for entry in self.control_stack.iter_mut().rev() {
            if let ControlStackEntry::Frame(frame) = entry {
                return frame;
            }
        }
        panic!("no call frame");
    }
}

pub struct WasmFrame {
    pub locals: Box<[WasmValue]>,
    pub winst_id: WasmInstanceAddr,
}

pub struct WasmModuleInst {
    pub wmod: Rc<WasmModule>,
    pub funcaddrs: Box<[WasmFuncAddr]>,
    pub tableaddrs: Box<[WasmTableAddr]>,
    pub memaddrs: Box<[WasmMemAddr]>,
    pub globaladdrs: Box<[WasmGlobalAddr]>,
    pub elemaddrs: Box<[WasmElemAddr]>,
    pub dataaddrs: Box<[WasmDataAddr]>,
    pub exports: Box<[WasmExternVal]>,
}

impl WasmModuleInst {
    pub fn resolve_export_by_name(&self, name: &str) -> Option<WasmExternVal> {
        for (i, export) in self.wmod.exports.iter().enumerate() {
            if export.name.0.as_ref() == name {
                return Some(self.exports[i]);
            }
        }
        None
    }

    pub fn resolve_export_fn_by_name(&self, name: &str) -> Option<WasmFuncAddr> {
        let externval = self.resolve_export_by_name(name);
        if let Some(WasmExternVal::Func(funcaddr)) = externval {
            Some(funcaddr)
        } else {
            None
        }
    }

    pub fn resolve_export_global_by_name(&self, name: &str) -> Option<WasmGlobalAddr> {
        let externval = self.resolve_export_by_name(name);
        if let Some(WasmExternVal::Global(globaladdr)) = externval {
            Some(globaladdr)
        } else {
            None
        }
    }
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

impl WasmModuleInst {
    pub fn addr_of<I: WasmIdx>(&self, idx: I) -> I::Addr {
        idx.resolve_addr(self)
    }
}

#[derive(Debug, Copy, Clone)]
pub enum WasmExternVal {
    Func(WasmFuncAddr),
    Table(WasmTableAddr),
    Mem(WasmMemAddr),
    Global(WasmGlobalAddr),
}

impl WasmExternVal {
    pub fn kind(&self) -> WasmExternValKind {
        match self {
            WasmExternVal::Func(_) => WasmExternValKind::Func,
            WasmExternVal::Table(_) => WasmExternValKind::Table,
            WasmExternVal::Mem(_) => WasmExternValKind::Mem,
            WasmExternVal::Global(_) => WasmExternValKind::Global,
        }
    }
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum WasmExternValKind {
    Func,
    Table,
    Mem,
    Global,
}

pub struct ModuleRef<T: ?Sized>(*const T);

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
    ) -> Result<DynamicWasmResult, WasmTrap> {
        let func = self.funcs.resolve(funcaddr);
        let ty = func.type_;
        match func.impl_ {
            WasmFuncImpl::Wasm { winst_id, func } => {
                let mut stack = WasmStack::new();
                let mut locals = args.into_vec();
                // todo: typecheck args
                for local_type in &func.locals {
                    locals.push(WasmValue::default_of_type(local_type));
                }
                stack.push_frame(WasmFrame {
                    locals: locals.into_boxed_slice(),
                    winst_id,
                });
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

pub type WasmHostFunc =
    &'static dyn Fn(&mut WasmStore, WasmInstanceAddr, &[WasmValue]) -> Box<[WasmValue]>;

pub struct WasmResult(pub Vec<WasmValue>);

pub struct DynamicWasmResult {
    pub ty: Box<[WasmValueType]>,
    pub res: WasmResult,
}

impl DynamicWasmResult {
    pub fn void() -> Self {
        DynamicWasmResult {
            ty: Box::new([]),
            res: WasmResult(vec![]),
        }
    }
}

impl std::fmt::Debug for DynamicWasmResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for DynamicWasmResult {
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
                WasmValueType::Vec(_vect) => {
                    todo!()
                }
                WasmValueType::Ref(reft) => match reft {
                    WasmRefType::ExternRef => write!(f, "{}", unsafe { val.ref_.extern_.0 })?,
                    WasmRefType::FuncRef => write!(f, "{}", unsafe { val.ref_.func })?,
                },
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

fn wasm_value_eq(ty: &WasmValueType, v1: &WasmValue, v2: &WasmValue) -> bool {
    match ty {
        WasmValueType::Num(WasmNumType::I32) => unsafe { v1.num.i32 == v2.num.i32 },
        WasmValueType::Num(WasmNumType::I64) => unsafe { v1.num.i64 == v2.num.i64 },
        WasmValueType::Num(WasmNumType::F32) => unsafe {
            (v1.num.f32.is_nan() && v2.num.f32.is_nan()) || v1.num.f32 == v2.num.f32
        },
        WasmValueType::Num(WasmNumType::F64) => unsafe {
            (v1.num.f64.is_nan() && v2.num.f64.is_nan()) || v1.num.f64 == v2.num.f64
        },
        WasmValueType::Ref(WasmRefType::ExternRef) => unsafe {
            v1.ref_.extern_.0 == v2.ref_.extern_.0
        },
        WasmValueType::Ref(WasmRefType::FuncRef) => unsafe { v1.ref_.func == v2.ref_.func },
        WasmValueType::Vec(_wasm_vec_type) => todo!(),
    }
}

impl PartialEq for DynamicWasmResult {
    fn eq(&self, other: &Self) -> bool {
        if self.ty != other.ty {
            return false;
        }
        for ((v1, v2), ty) in self
            .res
            .0
            .iter()
            .zip(other.res.0.iter())
            .zip(self.ty.iter())
        {
            if !wasm_value_eq(ty, v1, v2) {
                return false;
            }
        }
        true
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

impl WasmValue {
    pub fn default_of_type(value_type: &WasmValueType) -> Self {
        use WasmValueType::*;
        match value_type {
            Num(WasmNumType::F32) => 0f32.into(),
            Num(WasmNumType::F64) => 0f64.into(),
            Num(WasmNumType::I32) => 0i32.into(),
            Num(WasmNumType::I64) => 0i64.into(),
            Ref(_) => WasmRefValue::NULL.into(),
            Vec(_) => WasmValue { vec: 0 },
        }
    }
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
pub struct WasmExternAddr(pub u32);

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
