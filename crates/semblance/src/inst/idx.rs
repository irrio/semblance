use crate::{
    inst::{
        WasmModuleInst,
        table::{
            WasmDataAddr, WasmElemAddr, WasmFuncAddr, WasmGlobalAddr, WasmMemAddr, WasmTableAddr,
        },
    },
    module::{WasmDataIdx, WasmElemIdx, WasmFuncIdx, WasmGlobalIdx, WasmMemIdx, WasmTableIdx},
};

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
