use std::rc::Rc;

use crate::{
    exec::exec,
    inst::{WasmRefValue, WasmStack, WasmTrap},
    module::{
        WasmData, WasmDataIdx, WasmDataMode, WasmElemIdx, WasmElemMode, WasmExportDesc, WasmExpr,
        WasmFunc, WasmFuncType, WasmGlobalType, WasmImportDesc, WasmInstructionRepr, WasmLimits,
        WasmMemType, WasmRefType, WasmTableType,
    },
};

use super::{
    ModuleRef, WasmDataInst, WasmElemInst, WasmExternVal, WasmFrame, WasmFuncImpl, WasmFuncInst,
    WasmGlobalInst, WasmMemInst, WasmModule, WasmModuleInst, WasmStore, WasmTableInst, WasmValue,
    table::{
        WasmDataAddr, WasmElemAddr, WasmFuncAddr, WasmGlobalAddr, WasmInstanceAddr, WasmMemAddr,
        WasmTableAddr,
    },
};

#[derive(Debug)]
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
    ConstExprTrapped(WasmTrap),
    StartFunctionTrapped(WasmTrap),
}

pub type WasmInstantiationResult<T = ()> = Result<T, WasmInstantiationError>;

struct ExternValCounts {
    funcs: usize,
    tables: usize,
    mems: usize,
    globals: usize,
}

fn count_externvals(externvals: &[WasmExternVal]) -> ExternValCounts {
    let mut counts = ExternValCounts {
        funcs: 0,
        tables: 0,
        mems: 0,
        globals: 0,
    };
    for externval in externvals {
        match externval {
            WasmExternVal::Func(_) => counts.funcs += 1,
            WasmExternVal::Global(_) => counts.globals += 1,
            WasmExternVal::Mem(_) => counts.mems += 1,
            WasmExternVal::Table(_) => counts.tables += 1,
        }
    }
    counts
}

fn exec_with_auxiliary_frame(
    winst_id: WasmInstanceAddr,
    stack: &mut WasmStack,
    store: &mut WasmStore,
    expr: &WasmExpr,
) -> Result<(), WasmTrap> {
    stack.push_frame(WasmFrame {
        locals: Box::new([]),
        winst_id,
    })?;
    exec(stack, store, expr)
}

impl WasmStore {
    pub fn instantiate(
        &mut self,
        wmod: Rc<WasmModule>,
        externvals: &[WasmExternVal],
    ) -> WasmInstantiationResult<WasmInstanceAddr> {
        typecheck_externvals(self, wmod.as_ref(), externvals)?;
        let counts = count_externvals(externvals);
        let winst_id = self.alloc_inst(wmod.clone());
        let mut funcaddrs = Vec::with_capacity(counts.funcs + wmod.funcs.len());
        funcaddrs.extend(externvals.iter().filter_map(|e| match e {
            WasmExternVal::Func(funcaddr) => Some(funcaddr),
            _ => None,
        }));
        for func in &wmod.funcs {
            funcaddrs.push(self.alloc_func(winst_id, &wmod, func));
        }
        let funcaddrs = funcaddrs.into_boxed_slice();
        let winst_init = build_auxiliary_inst(externvals, funcaddrs);
        let globalinit = eval_global_initializers(self, &winst_init, wmod.as_ref());
        let refinit = eval_element_segment_initializers(self, &winst_init, wmod.as_ref());

        let WasmModuleInst {
            funcaddrs,
            wmod: _,
            tableaddrs: _,
            memaddrs: _,
            globaladdrs: _,
            elemaddrs: _,
            dataaddrs: _,
            exports: _,
        } = winst_init;

        self.alloc_module(
            &wmod, winst_id, externvals, globalinit, refinit, funcaddrs, &counts,
        );
        let mut stack = WasmStack::new(1024);

        for (i, elem) in wmod.elems.iter().enumerate() {
            match &elem.elem_mode {
                WasmElemMode::Active {
                    table_idx,
                    offset_expr,
                } => {
                    let n = elem.init.len();
                    use WasmInstructionRepr::*;
                    let expr = [
                        I32Const { val: 0 },
                        I32Const { val: n as i32 },
                        TableInit {
                            table_idx: *table_idx,
                            elem_idx: WasmElemIdx(i as u32),
                        },
                        ElemDrop {
                            elem_idx: WasmElemIdx(i as u32),
                        },
                        ExprEnd,
                    ];
                    exec_with_auxiliary_frame(winst_id, &mut stack, self, &offset_expr)
                        .map_err(WasmInstantiationError::ConstExprTrapped)?;
                    exec_with_auxiliary_frame(winst_id, &mut stack, self, &expr)
                        .map_err(WasmInstantiationError::ConstExprTrapped)?;
                }
                WasmElemMode::Declarative => {
                    use WasmInstructionRepr::*;
                    let expr = [
                        ElemDrop {
                            elem_idx: WasmElemIdx(i as u32),
                        },
                        ExprEnd,
                    ];
                    exec_with_auxiliary_frame(winst_id, &mut stack, self, &expr)
                        .map_err(WasmInstantiationError::ConstExprTrapped)?;
                }
                _ => continue,
            }
        }

        for (i, data) in wmod.datas.iter().enumerate() {
            match &data.mode {
                WasmDataMode::Active {
                    mem_idx: _,
                    offset_expr,
                } => {
                    let n = data.bytes.len();
                    use WasmInstructionRepr::*;
                    let expr = [
                        I32Const { val: 0 },
                        I32Const { val: n as i32 },
                        MemoryInit {
                            data_idx: WasmDataIdx(i as u32),
                        },
                        DataDrop {
                            data_idx: WasmDataIdx(i as u32),
                        },
                        ExprEnd,
                    ];
                    exec_with_auxiliary_frame(winst_id, &mut stack, self, &offset_expr)
                        .map_err(WasmInstantiationError::ConstExprTrapped)?;
                    exec_with_auxiliary_frame(winst_id, &mut stack, self, &expr)
                        .map_err(WasmInstantiationError::ConstExprTrapped)?;
                }
                _ => continue,
            }
        }

        if let Some(func_idx) = wmod.start {
            use WasmInstructionRepr::*;
            let expr = [Call { func_idx }, ExprEnd];
            exec_with_auxiliary_frame(winst_id, &mut stack, self, &expr)
                .map_err(WasmInstantiationError::StartFunctionTrapped)?;
        }

        Ok(winst_id)
    }

    fn alloc_module(
        &mut self,
        wmod: &WasmModule,
        winst_id: WasmInstanceAddr,
        externvals: &[WasmExternVal],
        globalinit: Box<[WasmValue]>,
        refinit: Box<[Box<[WasmRefValue]>]>,
        funcaddrs: Box<[WasmFuncAddr]>,
        counts: &ExternValCounts,
    ) {
        self.instances.resolve_mut(winst_id).funcaddrs = funcaddrs;

        let mut tableaddrs = Vec::with_capacity(counts.tables + wmod.tables.len());
        tableaddrs.extend(externvals.iter().filter_map(|e| match e {
            WasmExternVal::Table(tableaddr) => Some(tableaddr),
            _ => None,
        }));
        for table in &wmod.tables {
            tableaddrs.push(self.alloc_table(table));
        }
        self.instances.resolve_mut(winst_id).tableaddrs = tableaddrs.into_boxed_slice();

        let mut memaddrs = Vec::with_capacity(counts.mems + wmod.mems.len());
        memaddrs.extend(externvals.iter().filter_map(|e| match e {
            WasmExternVal::Mem(memaddr) => Some(memaddr),
            _ => None,
        }));
        for mem in &wmod.mems {
            memaddrs.push(self.alloc_mem(mem));
        }
        self.instances.resolve_mut(winst_id).memaddrs = memaddrs.into_boxed_slice();

        let mut globaladdrs = Vec::with_capacity(counts.globals + wmod.globals.len());
        globaladdrs.extend(externvals.iter().filter_map(|e| match e {
            WasmExternVal::Global(globaladdr) => Some(globaladdr),
            _ => None,
        }));
        for (global, init) in wmod.globals.iter().zip(globalinit) {
            globaladdrs.push(self.alloc_global(&global.global_type, init));
        }
        self.instances.resolve_mut(winst_id).globaladdrs = globaladdrs.into_boxed_slice();

        let mut elemaddrs = Vec::with_capacity(wmod.elems.len());
        for (elem, init) in wmod.elems.iter().zip(refinit) {
            elemaddrs.push(self.alloc_elem(elem.ref_type, init));
        }
        self.instances.resolve_mut(winst_id).elemaddrs = elemaddrs.into_boxed_slice();

        let mut dataaddrs = Vec::with_capacity(wmod.datas.len());
        for data in &wmod.datas {
            dataaddrs.push(self.alloc_data(data));
        }
        self.instances.resolve_mut(winst_id).dataaddrs = dataaddrs.into_boxed_slice();

        let mut exports = Vec::with_capacity(wmod.exports.len());
        for wexp in &wmod.exports {
            exports.push(self.resolve_export(&self.instances.resolve(winst_id), &wexp.desc));
        }
        self.instances.resolve_mut(winst_id).exports = exports.into_boxed_slice();
    }

    fn alloc_inst(&mut self, wmod: Rc<WasmModule>) -> WasmInstanceAddr {
        self.instances.add(WasmModuleInst {
            wmod,
            funcaddrs: Box::new([]),
            tableaddrs: Box::new([]),
            memaddrs: Box::new([]),
            globaladdrs: Box::new([]),
            elemaddrs: Box::new([]),
            dataaddrs: Box::new([]),
            exports: Box::new([]),
        })
    }

    fn alloc_func(
        &mut self,
        winst_id: WasmInstanceAddr,
        wmod: &WasmModule,
        func: &WasmFunc,
    ) -> WasmFuncAddr {
        self.funcs.add(WasmFuncInst {
            type_: ModuleRef(&wmod.types[func.type_idx.0 as usize]),
            impl_: WasmFuncImpl::Wasm {
                winst_id,
                func: ModuleRef(func),
            },
        })
    }

    fn alloc_table(&mut self, table: &WasmTableType) -> WasmTableAddr {
        self.tables.add(WasmTableInst {
            type_: ModuleRef(table),
            elems: vec![WasmRefValue::NULL; table.limits.min as usize],
        })
    }

    fn alloc_mem(&mut self, mem: &WasmMemType) -> WasmMemAddr {
        self.mems.add(WasmMemInst {
            type_: ModuleRef(mem),
            data: vec![0; mem.limits.min as usize * WasmMemInst::PAGE_SIZE],
        })
    }

    fn alloc_global(&mut self, global: &WasmGlobalType, init: WasmValue) -> WasmGlobalAddr {
        self.globals.add(WasmGlobalInst {
            type_: ModuleRef(global),
            val: init,
        })
    }

    fn alloc_elem(&mut self, elem: WasmRefType, init: Box<[WasmRefValue]>) -> WasmElemAddr {
        self.elems.add(WasmElemInst {
            type_: elem,
            elem: init,
        })
    }

    fn alloc_data(&mut self, data: &WasmData) -> WasmDataAddr {
        self.datas.add(WasmDataInst {
            data: Some(ModuleRef(data.bytes.as_ref())),
        })
    }

    fn resolve_export(&self, winst: &WasmModuleInst, desc: &WasmExportDesc) -> WasmExternVal {
        use WasmExportDesc::*;
        match desc {
            Func(func_idx) => WasmExternVal::Func(winst.addr_of(*func_idx)),
            Table(table_idx) => WasmExternVal::Table(winst.addr_of(*table_idx)),
            Global(global_idx) => WasmExternVal::Global(winst.addr_of(*global_idx)),
            Mem(mem_idx) => WasmExternVal::Mem(winst.addr_of(*mem_idx)),
        }
    }
}

fn typecheck_externvals(
    store: &WasmStore,
    wmod: &WasmModule,
    externvals: &[WasmExternVal],
) -> WasmInstantiationResult {
    if wmod.imports.len() != externvals.len() {
        return Err(WasmInstantiationError::ExternValArity {
            expected: wmod.imports.len(),
            actual: externvals.len(),
        });
    }
    for (externval, externtype) in externvals.iter().zip(wmod.imports.iter()) {
        typecheck_externval(store, wmod, externval, &externtype.desc)?;
    }
    Ok(())
}

fn typecheck_externval(
    store: &WasmStore,
    wmod: &WasmModule,
    externval: &WasmExternVal,
    wimp: &WasmImportDesc,
) -> WasmInstantiationResult {
    match (externval, wimp) {
        (WasmExternVal::Func(funcaddr), WasmImportDesc::Func(typeidx)) => {
            let ftype = &wmod.types[typeidx.0 as usize];
            let etype = &store
                .funcs
                .try_resolve(*funcaddr)
                .ok_or(WasmInstantiationError::InvalidExternFunc)?
                .type_;
            match_functype(etype, ftype)
        }
        (WasmExternVal::Global(globaladdr), WasmImportDesc::Global(globaltype)) => {
            let etype = &store
                .globals
                .try_resolve(*globaladdr)
                .ok_or(WasmInstantiationError::InvalidGlobalAddr)?
                .type_;
            match_globaltype(etype, globaltype)
        }
        (WasmExternVal::Mem(memaddr), WasmImportDesc::Mem(memtype)) => {
            let mem = &store
                .mems
                .try_resolve(*memaddr)
                .ok_or(WasmInstantiationError::InvalidMemAddr)?;
            let etype = mem.type_;
            let actual_size = mem.data.len() / WasmMemInst::PAGE_SIZE;
            match_memtype(&etype, memtype, actual_size)
        }
        (WasmExternVal::Table(tableaddr), WasmImportDesc::Table(tabletype)) => {
            let externtable = &store
                .tables
                .try_resolve(*tableaddr)
                .ok_or(WasmInstantiationError::InvalidTableAddr)?;
            let actual_size = externtable.elems.len();
            match_tabletype(&externtable.type_, tabletype, actual_size)
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

fn match_memtype(
    externtype: &WasmMemType,
    memtype: &WasmMemType,
    actual_size: usize,
) -> WasmInstantiationResult {
    if match_limits(&externtype.limits, &memtype.limits, actual_size) {
        Ok(())
    } else {
        Err(WasmInstantiationError::InvalidExternMem)
    }
}

fn match_tabletype(
    externtype: &WasmTableType,
    tabletype: &WasmTableType,
    actual_size: usize,
) -> WasmInstantiationResult {
    if externtype.ref_type == tabletype.ref_type
        && match_limits(&externtype.limits, &tabletype.limits, actual_size)
    {
        Ok(())
    } else {
        Err(WasmInstantiationError::InvalidExternTable)
    }
}

fn match_limits(externlimits: &WasmLimits, limits: &WasmLimits, actual_size: usize) -> bool {
    let externmin = externlimits.min.max(actual_size as u32);
    if externmin >= limits.min {
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

fn build_auxiliary_inst(
    externvals: &[WasmExternVal],
    funcaddrs: Box<[WasmFuncAddr]>,
) -> WasmModuleInst {
    let winst = WasmModuleInst {
        wmod: Rc::new(WasmModule::empty()),
        funcaddrs: funcaddrs,
        tableaddrs: Box::new([]),
        memaddrs: Box::new([]),
        globaladdrs: externvals
            .iter()
            .filter_map(|externval| match externval {
                WasmExternVal::Global(globaladdr) => Some(*globaladdr),
                _ => None,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        elemaddrs: Box::new([]),
        dataaddrs: Box::new([]),
        exports: Box::new([]),
    };
    winst
}

fn eval_global_initializers(
    store: &WasmStore,
    winst: &WasmModuleInst,
    wmod: &WasmModule,
) -> Box<[WasmValue]> {
    let mut globalinit = Vec::with_capacity(wmod.globals.len());
    for global in &wmod.globals {
        let val = eval_const_expr(store, winst, &global.init);
        globalinit.push(val);
    }
    globalinit.into_boxed_slice()
}

fn eval_element_segment_initializers(
    store: &WasmStore,
    winst: &WasmModuleInst,
    wmod: &WasmModule,
) -> Box<[Box<[WasmRefValue]>]> {
    let mut refinit = Vec::with_capacity(wmod.elems.len());
    for elem in &wmod.elems {
        let mut vec = Vec::with_capacity(elem.init.len());
        for expr in &elem.init {
            let val = eval_const_expr(store, winst, expr);
            vec.push(unsafe { val.ref_ });
        }
        refinit.push(vec.into_boxed_slice());
    }
    refinit.into_boxed_slice()
}

fn eval_const_expr(store: &WasmStore, winst: &WasmModuleInst, expr: &WasmExpr) -> WasmValue {
    let mut out: WasmValue = 0.into();
    exec_const_expr(store, winst, expr, &mut out);
    out
}

fn exec_const_expr(
    store: &WasmStore,
    winst: &WasmModuleInst,
    expr: &WasmExpr,
    out: &mut WasmValue,
) {
    use WasmInstructionRepr::*;
    for instr in expr {
        match instr {
            I32Const { val } => *out = (*val).into(),
            I64Const { val } => *out = (*val).into(),
            F32Const { val } => *out = (*val).into(),
            F64Const { val } => *out = (*val).into(),
            RefNull { ref_type: _ } => *out = WasmRefValue::NULL.into(),
            RefFunc { func_idx } => {
                let funcaddr = winst.funcaddrs[func_idx.0 as usize];
                *out = WasmRefValue { func: funcaddr }.into();
            }
            GlobalGet { global_idx } => {
                let globaladdr = winst.globaladdrs[global_idx.0 as usize];
                *out = store.globals.resolve(globaladdr).val;
            }
            ExprEnd => break,
            _ => panic!("expr not const"),
        }
    }
}
