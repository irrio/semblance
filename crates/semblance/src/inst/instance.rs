use std::rc::Rc;

use crate::{
    inst::{
        externval::WasmExternVal,
        idx::WasmIdx,
        table::{
            WasmDataAddr, WasmElemAddr, WasmFuncAddr, WasmGlobalAddr, WasmMemAddr, WasmTableAddr,
        },
    },
    module::WasmModule,
};

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

impl WasmModuleInst {
    pub fn addr_of<I: WasmIdx>(&self, idx: I) -> I::Addr {
        idx.resolve_addr(self)
    }
}
