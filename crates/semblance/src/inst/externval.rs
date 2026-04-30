use crate::inst::table::{WasmFuncAddr, WasmGlobalAddr, WasmMemAddr, WasmTableAddr};

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
