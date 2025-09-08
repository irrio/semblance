use crate::{
    exec::exec,
    inst::{WasmRefValue, WasmStack},
    module::{
        WasmData, WasmDataIdx, WasmDataMode, WasmElemIdx, WasmElemMode, WasmExportDesc, WasmExpr,
        WasmFunc, WasmFuncType, WasmGlobalType, WasmImportDesc, WasmInstruction, WasmLimits,
        WasmMemType, WasmRefType, WasmTableType,
    },
};

use super::{
    WasmDataInst, WasmElemInst, WasmExportInst, WasmExternVal, WasmFrame, WasmFuncImpl,
    WasmFuncInst, WasmGlobalInst, WasmMemInst, WasmModule, WasmModuleInst, WasmStore,
    WasmTableInst, WasmValue,
    table::{
        WasmDataAddr, WasmElemAddr, WasmFuncAddr, WasmGlobalAddr, WasmInstanceAddr, WasmMemAddr,
        WasmTableAddr,
    },
};

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

impl<'wmod> WasmStore<'wmod> {
    pub fn instantiate(
        &mut self,
        wmod: &'wmod WasmModule,
        externvals: &[WasmExternVal],
    ) -> WasmInstantiationResult<WasmInstanceAddr> {
        typecheck_externvals(self, wmod, externvals)?;
        let winst_init = build_auxiliary_inst(externvals);
        let globalinit = eval_global_initializers(self, &winst_init, wmod);
        let refinit = eval_element_segment_initializers(self, &winst_init, wmod);

        let winst_id = self.alloc_module(wmod, externvals, globalinit, refinit);

        let mut stack = WasmStack::new();
        stack.push_frame(WasmFrame {
            arity: 0,
            locals: Box::new([]),
            winst_id,
        });

        for (i, elem) in wmod.elems.iter().enumerate() {
            match &elem.elem_mode {
                WasmElemMode::Active {
                    table_idx,
                    offset_expr,
                } => {
                    let n = elem.init.len();
                    use WasmInstruction::*;
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
                    exec(&mut stack, self, &offset_expr.0);
                    exec(&mut stack, self, &expr);
                }
                WasmElemMode::Declarative => {
                    use WasmInstruction::*;
                    let expr = [
                        ElemDrop {
                            elem_idx: WasmElemIdx(i as u32),
                        },
                        ExprEnd,
                    ];
                    exec(&mut stack, self, &expr);
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
                    use WasmInstruction::*;
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
                    exec(&mut stack, self, &offset_expr.0);
                    exec(&mut stack, self, &expr);
                }
                _ => continue,
            }
        }

        if let Some(func_idx) = wmod.start {
            use WasmInstruction::*;
            let expr = [Call { func_idx }, ExprEnd];
            exec(&mut stack, self, &expr);
        }

        stack.pop_frame();

        Ok(winst_id)
    }

    fn alloc_module(
        &mut self,
        wmod: &'wmod WasmModule,
        externvals: &[WasmExternVal],
        globalinit: Box<[WasmValue]>,
        refinit: Box<[Box<[WasmRefValue]>]>,
    ) -> WasmInstanceAddr {
        let counts = count_externvals(externvals);
        let winst_id = self.alloc_inst(wmod);

        let mut funcaddrs = Vec::with_capacity(counts.funcs + wmod.funcs.len());
        funcaddrs.extend(externvals.iter().filter_map(|e| match e {
            WasmExternVal::Func(funcaddr) => Some(funcaddr),
            _ => None,
        }));
        for func in &wmod.funcs {
            funcaddrs.push(self.alloc_func(winst_id, wmod, func));
        }
        self.instances.resolve_mut(winst_id).funcaddrs = funcaddrs.into_boxed_slice();

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
            exports.push(WasmExportInst {
                name: &wexp.name.0,
                value: self.resolve_export(&self.instances.resolve(winst_id), &wexp.desc),
            });
        }
        self.instances.resolve_mut(winst_id).exports = exports.into_boxed_slice();

        winst_id
    }

    fn alloc_inst(&mut self, wmod: &'wmod WasmModule) -> WasmInstanceAddr {
        self.instances.add(WasmModuleInst {
            types: &wmod.types,
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
        wmod: &'wmod WasmModule,
        func: &'wmod WasmFunc,
    ) -> WasmFuncAddr {
        self.funcs.add(WasmFuncInst {
            type_: &wmod.types[func.type_idx.0 as usize],
            impl_: WasmFuncImpl::Wasm { winst_id, func },
        })
    }

    fn alloc_table(&mut self, table: &'wmod WasmTableType) -> WasmTableAddr {
        self.tables.add(WasmTableInst {
            type_: table,
            elems: vec![WasmRefValue::NULL; table.limits.min as usize],
        })
    }

    fn alloc_mem(&mut self, mem: &'wmod WasmMemType) -> WasmMemAddr {
        self.mems.add(WasmMemInst {
            type_: mem,
            data: vec![0; mem.limits.min as usize * WasmMemInst::PAGE_SIZE],
        })
    }

    fn alloc_global(&mut self, global: &'wmod WasmGlobalType, init: WasmValue) -> WasmGlobalAddr {
        self.globals.add(WasmGlobalInst {
            type_: global,
            val: init,
        })
    }

    fn alloc_elem(&mut self, elem: WasmRefType, init: Box<[WasmRefValue]>) -> WasmElemAddr {
        self.elems.add(WasmElemInst {
            type_: elem,
            elem: init,
        })
    }

    fn alloc_data(&mut self, data: &'wmod WasmData) -> WasmDataAddr {
        self.datas.add(WasmDataInst {
            data: Some(&data.bytes),
        })
    }

    fn resolve_export(
        &self,
        winst: &WasmModuleInst<'wmod>,
        desc: &WasmExportDesc,
    ) -> WasmExternVal {
        use WasmExportDesc::*;
        match desc {
            Func(func_idx) => WasmExternVal::Func(winst.addr_of(*func_idx)),
            Table(table_idx) => WasmExternVal::Table(winst.addr_of(*table_idx)),
            Global(global_idx) => WasmExternVal::Global(winst.addr_of(*global_idx)),
            Mem(mem_idx) => WasmExternVal::Mem(winst.addr_of(*mem_idx)),
        }
    }
}

fn typecheck_externvals<'wmod>(
    store: &WasmStore<'wmod>,
    wmod: &'wmod WasmModule,
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

fn typecheck_externval<'wmod>(
    store: &WasmStore<'wmod>,
    wmod: &'wmod WasmModule,
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
            let etype = &store
                .mems
                .try_resolve(*memaddr)
                .ok_or(WasmInstantiationError::InvalidMemAddr)?
                .type_;
            match_memtype(etype, memtype)
        }
        (WasmExternVal::Table(tableaddr), WasmImportDesc::Table(tabletype)) => {
            let etype = &store
                .tables
                .try_resolve(*tableaddr)
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

fn build_auxiliary_inst(externvals: &[WasmExternVal]) -> WasmModuleInst {
    let winst = WasmModuleInst {
        types: &[],
        funcaddrs: externvals
            .iter()
            .filter_map(|externval| match externval {
                WasmExternVal::Func(funcaddr) => Some(*funcaddr),
                _ => None,
            })
            .collect::<Vec<_>>()
            .into_boxed_slice(),
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

fn eval_global_initializers<'wmod>(
    store: &WasmStore<'wmod>,
    winst: &WasmModuleInst<'wmod>,
    wmod: &'wmod WasmModule,
) -> Box<[WasmValue]> {
    let mut globalinit = Vec::with_capacity(wmod.globals.len());
    for global in &wmod.globals {
        let val = eval_const_expr(store, winst, &global.init);
        globalinit.push(val);
    }
    globalinit.into_boxed_slice()
}

fn eval_element_segment_initializers<'wmod>(
    store: &WasmStore<'wmod>,
    winst: &WasmModuleInst<'wmod>,
    wmod: &'wmod WasmModule,
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

fn eval_const_expr<'wmod>(
    store: &WasmStore<'wmod>,
    winst: &WasmModuleInst<'wmod>,
    expr: &WasmExpr,
) -> WasmValue {
    let mut out: WasmValue = 0.into();
    exec_const_expr(store, winst, expr, &mut out);
    out
}

fn exec_const_expr<'wmod>(
    store: &WasmStore<'wmod>,
    winst: &WasmModuleInst<'wmod>,
    expr: &WasmExpr,
    out: &mut WasmValue,
) {
    use WasmInstruction::*;
    for instr in &expr.0 {
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
