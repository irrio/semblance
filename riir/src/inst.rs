use crate::module::*;

pub struct WasmStack<'wmod> {
    value_stack: Vec<WasmValue>,
    label_stack: Vec<WasmLabel<'wmod>>,
    call_stack: Vec<WasmFrame<'wmod>>,
}

impl<'wmod> WasmStack<'wmod> {
    pub fn new() -> Self {
        WasmStack {
            value_stack: Vec::new(),
            label_stack: Vec::new(),
            call_stack: Vec::new(),
        }
    }

    pub fn push_value(&mut self, val: WasmValue) {
        self.value_stack.push(val);
    }

    pub fn pop_value(&mut self) -> WasmValue {
        self.value_stack.pop().expect("value stack underflow")
    }

    pub fn push_label(&mut self, label: WasmLabel<'wmod>) {
        self.label_stack.push(label);
    }

    pub fn pop_label(&mut self) -> WasmLabel<'wmod> {
        self.label_stack.pop().expect("label stack underflow")
    }

    pub fn push_frame(&mut self, frame: WasmFrame<'wmod>) {
        self.call_stack.push(frame);
    }

    pub fn pop_frame(&mut self) -> WasmFrame<'wmod> {
        self.call_stack.pop().expect("call stack underflow")
    }
}

pub struct WasmFrame<'wmod> {
    pub arity: u32,
    pub locals: Box<[WasmValue]>,
    pub wmod: &'wmod WasmModule,
}

pub struct WasmLabel<'wmod> {
    pub arity: u32,
    pub instr: &'wmod [WasmInstruction],
}

pub struct WasmModuleInst<'wmod> {
    pub types: &'wmod [WasmFuncType],
    pub funcaddrs: Box<[WasmFuncAddr]>,
    pub tableaddrs: Box<[WasmTableAddr]>,
    pub memaddrs: Box<[WasmMemAddr]>,
    pub globaladdrs: Box<WasmGlobalAddr>,
    pub elemaddrs: Box<WasmElemAddr>,
    pub dataaddrs: Box<WasmDataAddr>,
    pub exports: Box<WasmExportInst<'wmod>>,
}

pub struct WasmExportInst<'wmod> {
    pub name: &'wmod str,
    pub value: WasmExternVal,
}

pub enum WasmExternVal {
    Func(WasmFuncAddr),
    Table(WasmTableAddr),
    Mem(WasmMemAddr),
    Global(WasmGlobalAddr),
}

pub struct WasmStore<'winst, 'wmod> {
    pub funcs: Box<[WasmFuncInst<'winst, 'wmod>]>,
    pub tables: Box<[WasmTableInst<'wmod>]>,
    pub mems: Box<[WasmMemInst<'wmod>]>,
    pub globals: Box<[WasmGlobalInst<'wmod>]>,
    pub elems: Box<[WasmElemInst]>,
    pub datas: Box<[WasmDataInst<'wmod>]>,
}

pub struct WasmDataInst<'wmod> {
    pub data: &'wmod [u8],
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

pub struct WasmTableInst<'wmod> {
    pub type_: &'wmod WasmTableType,
    pub elems: Vec<WasmRefValue>,
}

pub struct WasmFuncInst<'winst, 'wmod> {
    pub type_: WasmFuncType,
    pub impl_: WasmFuncImpl<'winst, 'wmod>,
}

pub enum WasmFuncImpl<'winst, 'wmod> {
    Wasm {
        module: &'winst WasmModuleInst<'wmod>,
        func: &'wmod WasmFunc,
    },
    Host {
        hostfunc: WasmHostFunc,
    },
}

pub type WasmHostFunc = *const u8;

pub enum WasmResult {
    Ok(WasmValue),
    Trap,
}

#[derive(Clone, Copy)]
pub union WasmValue {
    pub num: WasmNumValue,
    pub vec: WasmVecValue,
    pub ref_: WasmRefValue,
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

#[derive(Copy, Clone)]
pub struct WasmFuncAddr(pub u32);

#[derive(Copy, Clone)]
pub struct WasmExternAddr(pub u32);

#[derive(Copy, Clone)]
pub struct WasmTableAddr(pub u32);

#[derive(Copy, Clone)]
pub struct WasmMemAddr(pub u32);

#[derive(Copy, Clone)]
pub struct WasmGlobalAddr(pub u32);

#[derive(Copy, Clone)]
pub struct WasmElemAddr(pub u32);

#[derive(Copy, Clone)]
pub struct WasmDataAddr(pub u32);
