use crate::module::WasmImportDesc;

use super::{WasmExternVal, WasmModule, WasmModuleInst, WasmStore};

pub enum WasmInstantiationError {
    ExternValArity { expected: usize, actual: usize },
    InvalidExternval,
}

pub type WasmInstantiationResult<T = ()> = Result<T, WasmInstantiationError>;

pub fn instantiate<'wmod>(
    wmod: &'wmod WasmModule,
    store: &mut WasmStore,
    externvals: &[WasmExternVal],
) -> WasmInstantiationResult<WasmModuleInst<'wmod>> {
    typecheck_externvals(wmod, store, externvals)?;

    todo!();
}

fn typecheck_externvals(
    wmod: &WasmModule,
    store: &WasmStore,
    externvals: &[WasmExternVal],
) -> WasmInstantiationResult {
    if wmod.imports.len() != externvals.len() {
        return Err(WasmInstantiationError::ExternValArity {
            expected: wmod.imports.len(),
            actual: externvals.len(),
        });
    }
    for (externval, externtype) in externvals.iter().zip(wmod.imports.iter()) {
        typecheck_externval(wmod, store, externval, &externtype.desc)?;
    }
    Ok(())
}

fn typecheck_externval(
    wmod: &WasmModule,
    store: &WasmStore,
    externval: &WasmExternVal,
    wimp: &WasmImportDesc,
) -> WasmInstantiationResult {
    match (externval, wimp) {
        (WasmExternVal::Func(funcaddr), WasmImportDesc::Func(typeidx)) => {
            todo!()
        }
        (WasmExternVal::Global(globaladdr), WasmImportDesc::Global(globaltype)) => {
            todo!()
        }
        (WasmExternVal::Mem(memaddr), WasmImportDesc::Mem(memtype)) => {
            todo!()
        }
        (WasmExternVal::Table(tableaddr), WasmImportDesc::Table(tabletype)) => {
            todo!()
        }
        _ => Err(WasmInstantiationError::InvalidExternval),
    }
}
