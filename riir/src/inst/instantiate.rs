use crate::module::{
    WasmFuncType, WasmGlobalType, WasmImportDesc, WasmLimits, WasmMemType, WasmTableType,
};

use super::{WasmExternVal, WasmModule, WasmModuleInst, WasmStore};

pub enum WasmInstantiationError {
    ExternValArity { expected: usize, actual: usize },
    InvalidFuncAddr,
    InvalidGlobalAddr,
    InvalidMemAddr,
    InvalidTableAddr,
    InvalidExternFunc,
    InvalidExternGlobal,
    InvalidExternMem,
    InvalidExternTable,
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
            let ftype = &wmod.types[typeidx.0 as usize];
            let etype = &store
                .funcs
                .get(funcaddr.0 as usize - 1)
                .ok_or(WasmInstantiationError::InvalidExternFunc)?
                .type_;
            match_functype(etype, ftype)
        }
        (WasmExternVal::Global(globaladdr), WasmImportDesc::Global(globaltype)) => {
            let etype = &store
                .globals
                .get(globaladdr.0 as usize - 1)
                .ok_or(WasmInstantiationError::InvalidGlobalAddr)?
                .type_;
            match_globaltype(etype, globaltype)
        }
        (WasmExternVal::Mem(memaddr), WasmImportDesc::Mem(memtype)) => {
            let etype = &store
                .mems
                .get(memaddr.0 as usize - 1)
                .ok_or(WasmInstantiationError::InvalidMemAddr)?
                .type_;
            match_memtype(etype, memtype)
        }
        (WasmExternVal::Table(tableaddr), WasmImportDesc::Table(tabletype)) => {
            let etype = &store
                .tables
                .get(tableaddr.0 as usize - 1)
                .ok_or(WasmInstantiationError::InvalidTableAddr)?
                .type_;
            match_tabletype(etype, tabletype)
        }
        _ => Err(WasmInstantiationError::InvalidExternval),
    }
}

fn match_functype(externtype: &WasmFuncType, functype: &WasmFuncType) -> WasmInstantiationResult {
    if externtype == functype {
        Ok(())
    } else {
        Err(WasmInstantiationError::InvalidExternFunc)
    }
}

fn match_globaltype(
    externtype: &WasmGlobalType,
    globaltype: &WasmGlobalType,
) -> WasmInstantiationResult {
    if externtype == globaltype {
        Ok(())
    } else {
        Err(WasmInstantiationError::InvalidExternGlobal)
    }
}

fn match_memtype(externtype: &WasmMemType, memtype: &WasmMemType) -> WasmInstantiationResult {
    if match_limits(&externtype.limits, &memtype.limits) {
        Ok(())
    } else {
        Err(WasmInstantiationError::InvalidExternMem)
    }
}

fn match_tabletype(
    externtype: &WasmTableType,
    tabletype: &WasmTableType,
) -> WasmInstantiationResult {
    if externtype.ref_type == tabletype.ref_type
        && match_limits(&externtype.limits, &tabletype.limits)
    {
        Ok(())
    } else {
        Err(WasmInstantiationError::InvalidExternTable)
    }
}

fn match_limits(externlimits: &WasmLimits, limits: &WasmLimits) -> bool {
    if externlimits.min >= limits.min {
        match limits.max {
            None => true,
            Some(max) => match externlimits.max {
                None => false,
                Some(externmax) => externmax <= max,
            },
        }
    } else {
        false
    }
}
