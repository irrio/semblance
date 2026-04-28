use std::{collections::HashMap, path::Path, rc::Rc};

use crate::{
    inst::{WasmExternVal, WasmHostFunc, WasmStore, instantiate::WasmInstantiationError},
    module::{WasmFuncType, WasmModule},
};

use super::depgraph::WasmDependencyGraph;

#[derive(Debug)]
#[allow(dead_code)]
pub enum WasmLinkError {
    UnknownModule(String),
    UnknownSymbol(String, String),
    UnableToInferModuleNameFromPath(String),
    DependencyInstantiation {
        modname: String,
        err: WasmInstantiationError,
    },
}

pub type WasmLinkResult<T> = Result<T, WasmLinkError>;

pub fn infer_module_name_from_path(path: &Path) -> WasmLinkResult<String> {
    path.file_prefix()
        .map(|os| os.to_string_lossy().to_string())
        .ok_or_else(|| {
            WasmLinkError::UnableToInferModuleNameFromPath(path.to_string_lossy().to_string())
        })
}

struct HostModule {
    funcs: HashMap<&'static str, (&'static WasmFuncType, WasmHostFunc)>,
}

enum LinkerEntry {
    Wasm(Rc<WasmModule>),
    Host(HostModule),
}

pub struct WasmLinker {
    modules: HashMap<String, LinkerEntry>,
}

impl WasmLinker {
    pub fn new() -> Self {
        WasmLinker {
            modules: HashMap::new(),
        }
    }

    pub fn add_host_module(
        &mut self,
        modname: String,
        hostfuncs: &[(&'static str, &'static WasmFuncType, WasmHostFunc)],
    ) {
        let mut funcs = HashMap::with_capacity(hostfuncs.len());
        for (name, functype, func) in hostfuncs {
            funcs.insert(*name, (*functype, *func));
        }
        self.modules
            .insert(modname, LinkerEntry::Host(HostModule { funcs }));
    }

    pub fn add_module(&mut self, modname: String, module: Rc<WasmModule>) {
        self.modules.insert(modname, LinkerEntry::Wasm(module));
    }

    pub fn link(&self, wmod: &WasmModule) -> WasmLinkResult<(WasmStore, Vec<WasmExternVal>)> {
        let mut store = WasmStore::new();
        let order = {
            let mut depgraph = WasmDependencyGraph::new();
            for (modname, entry) in &self.modules {
                match entry {
                    LinkerEntry::Wasm(module) => {
                        depgraph.add_module_deps(modname, module);
                    }
                    LinkerEntry::Host(_) => {
                        depgraph.add_deps(modname, Box::new([]));
                    }
                }
            }
            depgraph.add_module_deps("", wmod);
            let mut order = depgraph.toposort("");
            order.pop();
            order
        };
        let mut env = HashMap::<(&str, &str), WasmExternVal>::new();
        for modname in order {
            let entry = self
                .modules
                .get(modname)
                .ok_or_else(|| WasmLinkError::UnknownModule(modname.to_string()))?;
            match entry {
                LinkerEntry::Host(hostmod) => {
                    for (name, (functype, func)) in &hostmod.funcs {
                        let funcaddr = store.alloc_hostfunc(functype, *func);
                        env.insert((&modname, name), WasmExternVal::Func(funcaddr));
                    }
                }
                LinkerEntry::Wasm(wmod) => {
                    let mut externvals = Vec::with_capacity(wmod.imports.len());
                    for import in &wmod.imports {
                        externvals.push(
                            *env.get(&(&import.module_name.0, &import.item_name.0))
                                .ok_or_else(|| {
                                    WasmLinkError::UnknownSymbol(
                                        import.module_name.0.to_string(),
                                        import.item_name.0.to_string(),
                                    )
                                })?,
                        );
                    }
                    let winst_id = store.instantiate(wmod.clone(), &externvals).map_err(|e| {
                        WasmLinkError::DependencyInstantiation {
                            modname: modname.to_string(),
                            err: e,
                        }
                    })?;
                    let winst = store.instances.resolve(winst_id);
                    for (i, export) in wmod.exports.iter().enumerate() {
                        env.insert((modname, &export.name.0), winst.exports[i]);
                    }
                }
            }
        }
        let mut externvals = Vec::with_capacity(wmod.imports.len());
        for import in &wmod.imports {
            externvals.push(
                *env.get(&(&import.module_name.0, &import.item_name.0))
                    .ok_or_else(|| {
                        WasmLinkError::UnknownSymbol(
                            import.module_name.0.to_string(),
                            import.item_name.0.to_string(),
                        )
                    })?,
            );
        }
        Ok((store, externvals))
    }
}
