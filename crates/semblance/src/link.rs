use crate::{
    inst::{WasmExternVal, WasmHostFunc, WasmStore, instantiate::WasmInstantiationError},
    module::{WasmFuncType, WasmModule},
};
use std::{collections::HashMap, path::Path};

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
    funcs: HashMap<&'static str, (WasmFuncType, WasmHostFunc)>,
}

enum LinkerEntry {
    Wasm(WasmModule),
    Host(HostModule),
}

enum LinkerEntryRef<'a> {
    Wasm(&'a WasmModule),
    Host(&'a HostModule),
}

impl LinkerEntry {
    fn as_ref<'a>(&'a self) -> LinkerEntryRef<'a> {
        match self {
            LinkerEntry::Wasm(wasm) => LinkerEntryRef::Wasm(wasm),
            LinkerEntry::Host(host) => LinkerEntryRef::Host(host),
        }
    }
}

pub struct WasmLinker {
    modules: HashMap<String, LinkerEntry>,
}

impl<'l> WasmLinker {
    pub fn new() -> Self {
        WasmLinker {
            modules: HashMap::new(),
        }
    }

    pub fn with_host_module(
        mut self,
        modname: String,
        hostfuncs: &[(&'static str, WasmFuncType, WasmHostFunc)],
    ) -> Self {
        let mut funcs = HashMap::with_capacity(hostfuncs.len());
        for (name, functype, func) in hostfuncs {
            funcs.insert(*name, (functype.clone(), *func));
        }
        self.modules
            .insert(modname, LinkerEntry::Host(HostModule { funcs }));
        self
    }

    pub fn with_module(mut self, modname: String, module: WasmModule) -> Self {
        self.modules.insert(modname, LinkerEntry::Wasm(module));
        self
    }

    pub fn link<'wmod: 'l>(
        &'l self,
        wmod: &'wmod WasmModule,
    ) -> WasmLinkResult<(WasmStore<'l>, Vec<WasmExternVal>)> {
        let mut store = WasmStore::new();
        let order = {
            let mut depgraph = WasmDependencyGraph::new();
            for (modname, entry) in &self.modules {
                match entry {
                    LinkerEntry::Wasm(module) => {
                        depgraph.add_module_deps(modname.clone(), module);
                    }
                    LinkerEntry::Host(_) => {
                        depgraph.add_deps(modname.clone(), Box::new([]));
                    }
                }
            }
            depgraph.add_module_deps("".to_string(), wmod);
            let mut order = depgraph.topological_sort("".to_string());
            order.pop();
            order
        };
        let mut env = HashMap::<(&str, &str), WasmExternVal>::new();
        for modname in &order {
            let entry = if modname.is_empty() {
                LinkerEntryRef::Wasm(wmod)
            } else {
                self.modules
                    .get(modname)
                    .map(|e| e.as_ref())
                    .ok_or(WasmLinkError::UnknownModule(modname.clone()))?
            };
            match entry {
                LinkerEntryRef::Host(hostmod) => {
                    for (name, (functype, func)) in &hostmod.funcs {
                        let funcaddr = store.alloc_hostfunc(functype, *func);
                        env.insert((&modname, name), WasmExternVal::Func(funcaddr));
                    }
                }
                LinkerEntryRef::Wasm(wmod) => {
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
                    let winst_id = store.instantiate(wmod, &externvals).map_err(|e| {
                        WasmLinkError::DependencyInstantiation {
                            modname: modname.to_string(),
                            err: e,
                        }
                    })?;
                    for export in &store.instances.resolve(winst_id).exports {
                        env.insert((modname, export.name), export.value);
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

struct WasmDependencyGraph {
    deps: HashMap<String, Box<[String]>>,
}

impl WasmDependencyGraph {
    pub fn new() -> Self {
        WasmDependencyGraph {
            deps: HashMap::new(),
        }
    }

    pub fn add_deps(&mut self, modname: String, deps: Box<[String]>) {
        self.deps.insert(modname, deps);
    }

    pub fn add_module_deps(&mut self, modname: String, wmod: &WasmModule) {
        let deps = wmod
            .imports
            .iter()
            .map(|import| import.module_name.0.to_string())
            .collect::<Vec<_>>();
        self.add_deps(modname, deps.into_boxed_slice());
    }

    pub fn topological_sort(mut self, start: String) -> Vec<String> {
        let mut stack: Vec<String> = vec![start];
        let mut out = vec![];
        while let Some(modname) = stack.pop() {
            if let Some(deps) = self.deps.remove(&modname) {
                stack.push(modname);
                for dep in deps {
                    if self.deps.contains_key(&dep) {
                        stack.push(dep);
                    }
                }
            } else {
                out.push(modname);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_topo_sort() {
        let mut depgraph = WasmDependencyGraph::new();
        depgraph.add_deps(
            "a".to_string(),
            Box::new(["b".to_string(), "c".to_string()]),
        );
        depgraph.add_deps("b".to_string(), Box::new(["c".to_string()]));
        depgraph.add_deps("c".to_string(), Box::new([]));
        let order = depgraph.topological_sort("a".to_string());
        assert_eq!(
            order,
            vec!["c".to_string(), "b".to_string(), "a".to_string()]
        );
    }
}
