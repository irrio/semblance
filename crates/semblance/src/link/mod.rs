mod depgraph;
mod linker;

pub use linker::{WasmLinkError, WasmLinkResult, WasmLinker, infer_module_name_from_path};
