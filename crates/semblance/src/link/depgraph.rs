use std::collections::HashMap;

use crate::module::WasmModule;

pub struct WasmDependencyGraph<'s> {
    deps: HashMap<&'s str, Box<[&'s str]>>,
}

impl<'s> WasmDependencyGraph<'s> {
    pub fn new() -> Self {
        WasmDependencyGraph {
            deps: HashMap::new(),
        }
    }

    pub fn add_deps(&mut self, modname: &'s str, deps: Box<[&'s str]>) {
        self.deps.insert(modname, deps);
    }

    pub fn add_module_deps(&mut self, modname: &'s str, wmod: &'s WasmModule) {
        let deps = wmod
            .imports
            .iter()
            .map(|import| import.module_name.0.as_ref())
            .collect::<Vec<_>>();
        self.add_deps(modname, deps.into_boxed_slice());
    }

    pub fn toposort(mut self, start: &'s str) -> Vec<&'s str> {
        let mut stack = vec![start];
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
        depgraph.add_deps("a", Box::new(["b", "c"]));
        depgraph.add_deps("b", Box::new(["c"]));
        depgraph.add_deps("c", Box::new([]));
        let order = depgraph.toposort("a");
        assert_eq!(order, vec!["c", "b", "a"]);
    }
}
