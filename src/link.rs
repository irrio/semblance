use crate::{
    inst::{WasmExternVal, WasmHostFunc, WasmStore},
    module::{WasmImportDesc, WasmModule},
};
use std::collections::HashMap;

#[derive(Debug)]
#[allow(dead_code)]
pub enum WasmLinkError {
    UnknownModule(String),
    ModuleHasNoExportedSymbol(String, String),
}

pub type WasmLinkResult<T> = Result<T, WasmLinkError>;

pub struct WasmLinker {
    // modules: HashMap<String, WasmModule>,
    hostfuncs: HashMap<(&'static str, &'static str), WasmHostFunc>,
}

impl WasmLinker {
    pub fn new() -> Self {
        WasmLinker {
            // modules: HashMap::new(),
            hostfuncs: HashMap::new(),
        }
    }

    pub fn register_hostfunc(
        &mut self,
        modname: &'static str,
        funcname: &'static str,
        hostfunc: WasmHostFunc,
    ) {
        self.hostfuncs.insert((modname, funcname), hostfunc);
    }

    pub fn link<'wmod>(
        &self,
        wmod: &'wmod WasmModule,
    ) -> WasmLinkResult<(WasmStore<'wmod>, Vec<WasmExternVal>)> {
        let mut store = WasmStore::new();
        let mut externvals = Vec::with_capacity(wmod.imports.len());
        for import in &wmod.imports {
            if let WasmImportDesc::Func(typeidx) = import.desc {
                if let Some(hostfunc) = self
                    .hostfuncs
                    .get(&(&import.module_name.0, &import.item_name.0))
                {
                    let ty = &wmod.types[typeidx.0 as usize];
                    let funcaddr = store.alloc_hostfunc(ty, *hostfunc);
                    externvals.push(WasmExternVal::Func(funcaddr));
                } else {
                    return Err(WasmLinkError::UnknownModule(
                        import.module_name.0.to_string(),
                    ));
                }
            }
        }
        Ok((store, externvals))
    }
}
