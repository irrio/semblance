use super::*;

#[derive(Debug)]
pub enum WasmValidationError {
    InvalidTypeIdx(u32),
    InvalidGlobalIdx(u32),
    InvalidTableIdx(u32),
    InvalidMemIdx(u32),
    InvalidFuncIdx(u32),
    InvalidElemIdx(u32),
    InvalidDataIdx(u32),
    InvalidLocalIdx(u32),
    InvalidLabelIdx(u32),
    InvalidLimits {
        range: u32,
    },
    InvalidStartFunc,
    TooManyMemories,
    NoMemory,
    InvalidAlignment,
    ExprNotConst,
    DuplicateExportName(String),
    MismatchedType {
        expected: WasmValueType,
        actual: Option<WasmValueType>,
    },
    MismatchedTableCopy {
        src: WasmRefType,
        dst: WasmRefType,
    },
    MismatchedTableInit {
        table: WasmRefType,
        elem: WasmRefType,
    },
    TooManySelectTypes,
    InvalidReturn,
    InvalidCallIndirect,
}

pub type WasmValidationResult<T> = Result<T, WasmValidationError>;

use context::*;

pub fn validate(wmod: &WasmModule) -> WasmValidationResult<()> {
    // C'
    let mut wmod_ctx = ModuleContext::from_module(wmod)?;
    for table in &wmod.tables {
        validate_table(table, &wmod_ctx)?;
    }
    for mem in &wmod.mems {
        validate_mem(mem, &wmod_ctx)?;
    }
    for global in &wmod.globals {
        validate_global(global, &wmod_ctx)?;
    }
    for elem in &wmod.elems {
        validate_elem(elem, &wmod_ctx)?;
    }
    for data in &wmod.datas {
        validate_data(data, &wmod_ctx)?;
    }

    // C
    wmod_ctx.include_internal_globals(wmod);
    for func in &wmod.funcs {
        validate_func(func, &wmod_ctx)?;
    }
    if let Some(start) = wmod.start {
        validate_start_func(start, &wmod_ctx)?;
    }
    for import in &wmod.imports {
        validate_import(import, &wmod_ctx)?;
    }
    for export in &wmod.exports {
        validate_export(export, &wmod_ctx)?;
    }

    if wmod_ctx.memories.len() > 1 {
        return Err(WasmValidationError::TooManyMemories);
    }

    validate_export_names(&wmod)?;

    Ok(())
}

fn validate_export_names(wmod: &WasmModule) -> WasmValidationResult<()> {
    let mut names = std::collections::HashSet::new();
    for export in &wmod.exports {
        if names.contains(&export.name.0) {
            return Err(WasmValidationError::DuplicateExportName(
                export.name.0.to_string(),
            ));
        } else {
            names.insert(&export.name.0);
        }
    }
    Ok(())
}

fn validate_export(export: &WasmExport, wmod_ctx: &ModuleContext) -> WasmValidationResult<()> {
    match export.desc {
        WasmExportDesc::Func(func_idx) => {
            wmod_ctx
                .funcs
                .get(func_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidFuncIdx(func_idx.0))?;
            Ok(())
        }
        WasmExportDesc::Table(table_idx) => {
            wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            Ok(())
        }
        WasmExportDesc::Mem(mem_idx) => {
            wmod_ctx
                .memories
                .get(mem_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidMemIdx(mem_idx.0))?;
            Ok(())
        }
        WasmExportDesc::Global(global_idx) => {
            wmod_ctx
                .globals
                .get(global_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidGlobalIdx(global_idx.0))?;
            Ok(())
        }
    }
}

fn validate_import(import: &WasmImport, wmod_ctx: &ModuleContext) -> WasmValidationResult<()> {
    match import.desc {
        WasmImportDesc::Func(type_idx) => {
            let _type = wmod_ctx
                .types
                .get(type_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTypeIdx(type_idx.0))?;
            Ok(())
        }
        WasmImportDesc::Table(ref table_type) => {
            validate_table(table_type, wmod_ctx)?;
            Ok(())
        }
        WasmImportDesc::Mem(ref mem_type) => {
            validate_mem(mem_type, wmod_ctx)?;
            Ok(())
        }
        WasmImportDesc::Global(ref _global_type) => Ok(()),
    }
}

fn validate_start_func(start: WasmFuncIdx, wmod_ctx: &ModuleContext) -> WasmValidationResult<()> {
    let func = wmod_ctx
        .funcs
        .get(start.0 as usize)
        .ok_or(WasmValidationError::InvalidFuncIdx(start.0))?;
    if func.input_type.0.len() > 0 {
        return Err(WasmValidationError::InvalidStartFunc);
    }
    if func.output_type.0.len() > 0 {
        return Err(WasmValidationError::InvalidStartFunc);
    }
    Ok(())
}

fn validate_func(func: &WasmFunc, wmod_ctx: &ModuleContext) -> WasmValidationResult<()> {
    let (output_type, mut expr_ctx) = ExprContext::from_func(&wmod_ctx, func)?;
    validate_expr_with_result_type(&func.body, &output_type, &wmod_ctx, &mut expr_ctx)?;
    Ok(())
}

fn validate_data(data: &WasmData, wmod_ctx: &ModuleContext) -> WasmValidationResult<()> {
    match data.mode {
        WasmDataMode::Active {
            mem_idx,
            ref offset_expr,
        } => validate_active_data(mem_idx, offset_expr, wmod_ctx),
        _ => Ok(()),
    }
}

fn validate_active_data(
    mem_idx: WasmMemIdx,
    offset_expr: &WasmExpr,
    wmod_ctx: &ModuleContext,
) -> WasmValidationResult<()> {
    let _mem = wmod_ctx
        .memories
        .get(mem_idx.0 as usize)
        .ok_or(WasmValidationError::InvalidMemIdx(mem_idx.0))?;
    let output_type = WasmResultType(Box::new([t!(i32)]));
    validate_expr_with_result_type(
        offset_expr,
        &output_type,
        wmod_ctx,
        &mut ExprContext::with_return_type(WasmValueType::Num(WasmNumType::I32)),
    )?;
    validate_expr_is_const(offset_expr, wmod_ctx)
}

fn validate_elem(elem: &WasmElem, wmod_ctx: &ModuleContext) -> WasmValidationResult<()> {
    let output_type = WasmResultType(Box::new([t!(i32)]));
    for expr in &elem.init {
        validate_expr_with_result_type(
            expr,
            &output_type,
            wmod_ctx,
            &mut ExprContext::with_return_type(WasmValueType::Ref(elem.ref_type)),
        )?;
        validate_expr_is_const(expr, wmod_ctx)?;
    }
    match elem.elem_mode {
        WasmElemMode::Active {
            table_idx,
            ref offset_expr,
        } => validate_active_elem(table_idx, offset_expr, wmod_ctx),
        _ => Ok(()),
    }?;
    Ok(())
}

fn validate_active_elem(
    table_idx: WasmTableIdx,
    offset_expr: &WasmExpr,
    wmod_ctx: &ModuleContext,
) -> WasmValidationResult<()> {
    let _table = wmod_ctx
        .tables
        .get(table_idx.0 as usize)
        .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
    let output_type = WasmResultType(Box::new([t!(i32)]));
    validate_expr_with_result_type(
        offset_expr,
        &output_type,
        wmod_ctx,
        &mut ExprContext::with_return_type(t!(i32)),
    )?;
    validate_expr_is_const(offset_expr, wmod_ctx)
}

fn validate_global(global: &WasmGlobal, wmod_ctx: &ModuleContext) -> WasmValidationResult<()> {
    let output_type = WasmResultType(Box::new([global.global_type.val_type]));
    let mut expr_context = ExprContext::with_return_type(global.global_type.val_type);
    validate_expr_with_result_type(&global.init, &output_type, wmod_ctx, &mut expr_context)?;
    validate_expr_is_const(&global.init, wmod_ctx)
}

fn validate_mem(mem: &WasmMemType, _wmod_ctx: &ModuleContext) -> WasmValidationResult<()> {
    validate_limits_within_range(&mem.limits, u16::MAX as u32)
}

fn validate_table(table: &WasmTableType, _wmod_ctx: &ModuleContext) -> WasmValidationResult<()> {
    validate_limits_within_range(&table.limits, u32::MAX - 1)
}

fn validate_limits_within_range(limits: &WasmLimits, range: u32) -> WasmValidationResult<()> {
    if limits.min <= range {
        match limits.max {
            Some(max) => {
                if max <= range {
                    if limits.min <= max {
                        return Ok(());
                    }
                }
            }
            None => return Ok(()),
        };
    }
    Err(WasmValidationError::InvalidLimits { range })
}

fn bit_width(t: WasmValueType) -> u32 {
    match t {
        WasmValueType::Num(WasmNumType::I32) => 32,
        WasmValueType::Num(WasmNumType::I64) => 64,
        WasmValueType::Num(WasmNumType::F32) => 32,
        WasmValueType::Num(WasmNumType::F64) => 64,
        WasmValueType::Vec(WasmVecType::V128) => 128,
        _ => {
            unreachable!("bit_width on unsized type");
        }
    }
}

fn validate_alignment(
    memarg: &WasmMemArg,
    t: WasmValueType,
    bits: Option<u32>,
) -> WasmValidationResult<()> {
    if 2u32.pow(memarg.align) > (bits.unwrap_or_else(|| bit_width(t)) / 8) {
        return Err(WasmValidationError::InvalidAlignment);
    }
    Ok(())
}

fn validate_load_instr(
    wmod_ctx: &ModuleContext,
    memarg: &WasmMemArg,
    stack: &mut TypeStack,
    t: WasmValueType,
    bits: Option<u32>,
) -> WasmValidationResult<()> {
    let _mem = wmod_ctx
        .memories
        .get(0)
        .ok_or(WasmValidationError::NoMemory)?;
    validate_alignment(memarg, t, bits)?;
    stack.pop(t!(i32))?;
    stack.push(t);
    Ok(())
}

fn validate_store_instr(
    wmod_ctx: &ModuleContext,
    memarg: &WasmMemArg,
    stack: &mut TypeStack,
    t: WasmValueType,
    bits: Option<u32>,
) -> WasmValidationResult<()> {
    let _mem = wmod_ctx
        .memories
        .get(0)
        .ok_or(WasmValidationError::NoMemory)?;
    validate_alignment(memarg, t, bits)?;
    stack.pop(t)?;
    stack.pop(t!(i32))?;
    Ok(())
}

fn validate_block_type(
    block_type: &WasmBlockType,
    wmod_ctx: &ModuleContext,
) -> WasmValidationResult<WasmFuncType> {
    match block_type {
        WasmBlockType::TypeRef(type_idx) => {
            let t = wmod_ctx
                .types
                .get(type_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTypeIdx(type_idx.0))?;
            Ok(t.clone())
        }
        WasmBlockType::InlineType(Some(t)) => Ok(WasmFuncType {
            input_type: WasmResultType(Box::new([])),
            output_type: WasmResultType(Box::new([*t])),
        }),
        WasmBlockType::InlineType(None) => Ok(WasmFuncType {
            input_type: WasmResultType(Box::new([])),
            output_type: WasmResultType(Box::new([])),
        }),
    }
}

fn validate_instr(
    op: &WasmInstruction,
    wmod_ctx: &ModuleContext,
    expr_ctx: &mut ExprContext,
    stack: &mut TypeStack,
) -> WasmValidationResult<()> {
    use WasmInstruction::*;
    match op {
        // -- t.const -- //
        I32Const { .. } => {
            stack.push(t!(i32));
        }
        I64Const { .. } => {
            stack.push(t!(i64));
        }
        F32Const { .. } => {
            stack.push(t!(f32));
        }
        F64Const { .. } => {
            stack.push(t!(f64));
        }
        // -- t.unop -- //
        I32Clz | I32Ctz | I32Popcnt | I32Extend8S | I32Extend16S => {
            stack.pop(t!(i32))?;
            stack.push(t!(i32));
        }
        I64Clz | I64Ctz | I64Popcnt | I64Extend8S | I64Extend16S | I64Extend32S => {
            stack.pop(t!(i64))?;
            stack.push(t!(i64));
        }
        F32Abs | F32Neg | F32Sqrt | F32Ceil | F32Floor | F32Trunc | F32Nearest => {
            stack.pop(t!(f32))?;
            stack.push(t!(f32));
        }
        F64Abs | F64Neg | F64Sqrt | F64Ceil | F64Floor | F64Trunc | F64Nearest => {
            stack.pop(t!(f64))?;
            stack.push(t!(f64));
        }
        // -- t.binop -- //
        I32Add | I32Sub | I32Mul | I32DivS | I32DivU | I32RemS | I32RemU | I32And | I32Or
        | I32Xor | I32Shl | I32ShrS | I32ShrU | I32Rotl | I32Rotr => {
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
            stack.push(t!(i32));
        }
        I64Add | I64Sub | I64Mul | I64DivS | I64DivU | I64RemS | I64RemU | I64And | I64Or
        | I64Xor | I64Shl | I64ShrS | I64ShrU | I64Rotl | I64Rotr => {
            stack.pop(t!(i64))?;
            stack.pop(t!(i64))?;
            stack.push(t!(i64));
        }
        F32Add | F32Sub | F32Mul | F32Div | F32Min | F32Max | F32CopySign => {
            stack.pop(t!(f32))?;
            stack.pop(t!(f32))?;
            stack.push(t!(f32));
        }
        F64Add | F64Sub | F64Mul | F64Div | F64Min | F64Max | F64CopySign => {
            stack.pop(t!(f64))?;
            stack.pop(t!(f64))?;
            stack.push(t!(f64));
        }
        // -- t.testop -- //
        I32EqZ => {
            stack.pop(t!(i32))?;
            stack.push(t!(i32));
        }
        I64EqZ => {
            stack.pop(t!(i64))?;
            stack.push(t!(i32));
        }
        // -- t.relop -- //
        I32Eq | I32Neq | I32LtS | I32LtU | I32GtS | I32GtU | I32LeS | I32LeU | I32GeS | I32GeU => {
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
            stack.push(t!(i32));
        }
        I64Eq | I64Neq | I64LtS | I64LtU | I64GtS | I64GtU | I64LeS | I64LeU | I64GeS | I64GeU => {
            stack.pop(t!(i64))?;
            stack.pop(t!(i64))?;
            stack.push(t!(i32));
        }
        F32Eq | F32Neq | F32Lt | F32Gt | F32Le | F32Ge => {
            stack.pop(t!(f32))?;
            stack.pop(t!(f32))?;
            stack.push(t!(i32));
        }
        F64Eq | F64Neq | F64Lt | F64Gt | F64Le | F64Ge => {
            stack.pop(t!(f64))?;
            stack.pop(t!(f64))?;
            stack.push(t!(i32));
        }
        // -- t2.cvtop_t1_sx -- //
        I32WrapI64 => {
            stack.pop(t!(i64))?;
            stack.push(t!(i32));
        }
        I32TruncF32S | I32TruncF32U | I32TruncSatF32S | I32TruncSatF32U | I32ReinterpretF32 => {
            stack.pop(t!(f32))?;
            stack.push(t!(i32));
        }
        I32TruncF64S | I32TruncF64U | I32TruncSatF64S | I32TruncSatF64U => {
            stack.pop(t!(f64))?;
            stack.push(t!(i32));
        }
        I64ExtendI32S | I64ExtendI32U => {
            stack.pop(t!(i32))?;
            stack.push(t!(i64));
        }
        I64TruncF32S | I64TruncF32U | I64TruncSatF32S | I64TruncSatF32U => {
            stack.pop(t!(f32))?;
            stack.push(t!(i64));
        }
        I64TruncF64S | I64TruncF64U | I64TruncSatF64S | I64TruncSatF64U | I64ReinterpretF64 => {
            stack.pop(t!(f64))?;
            stack.push(t!(i64));
        }
        F32ConvertI32S | F32ConvertI32U | F32ReinterpretI32 => {
            stack.pop(t!(i32))?;
            stack.push(t!(f32));
        }
        F32ConvertI64S | F32ConvertI64U => {
            stack.pop(t!(i64))?;
            stack.push(t!(f32));
        }
        F32DemoteF64 => {
            stack.pop(t!(f64))?;
            stack.push(t!(f32));
        }
        F64ConvertI32S | F64ConvertI32U => {
            stack.pop(t!(i32))?;
            stack.push(t!(f64));
        }
        F64ConvertI64S | F64ConvertI64U | F64ReinterpretI64 => {
            stack.pop(t!(i64))?;
            stack.push(t!(f64));
        }
        F64PromoteF32 => {
            stack.pop(t!(f32))?;
            stack.push(t!(f64));
        }
        // -- reference instructions -- //
        RefNull { ref_type } => {
            stack.push(WasmValueType::Ref(*ref_type));
        }
        RefIsNull => {
            stack.pop_ref_type()?;
            stack.push(t!(i32));
        }
        RefFunc { func_idx } => {
            let _func = wmod_ctx
                .funcs
                .get(func_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidFuncIdx(func_idx.0))?;
            if !wmod_ctx.refs.contains(func_idx) {
                return Err(WasmValidationError::InvalidFuncIdx(func_idx.0));
            }
            stack.push(t!(funcref));
        }
        // -- parametric instructions -- //
        Drop => {
            stack.pop_any()?;
        }
        Select { value_types } => match value_types.len() {
            0 => {
                stack.pop(t!(i32))?;
                let t = stack.pop_num_or_vec()?;
                stack.pop(t)?;
                stack.push(t);
            }
            1 => {
                let t = value_types[0];
                stack.pop(t!(i32))?;
                stack.pop(t)?;
                stack.pop(t)?;
                stack.push(t);
            }
            _ => {
                return Err(WasmValidationError::TooManySelectTypes);
            }
        },
        // -- variable instructions -- //
        LocalGet { local_idx } => {
            let local_type = expr_ctx
                .locals
                .get(local_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidLocalIdx(local_idx.0))?;
            stack.push(*local_type);
        }
        LocalSet { local_idx } => {
            let local_type = expr_ctx
                .locals
                .get(local_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidLocalIdx(local_idx.0))?;
            stack.pop(*local_type)?;
        }
        LocalTee { local_idx } => {
            let local_type = *expr_ctx
                .locals
                .get(local_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidLocalIdx(local_idx.0))?;
            stack.pop(local_type)?;
            stack.push(local_type);
        }
        GlobalGet { global_idx } => {
            let global_type = wmod_ctx
                .globals
                .get(global_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidGlobalIdx(global_idx.0))?;
            stack.push(global_type.val_type);
        }
        GlobalSet { global_idx } => {
            let global_type = wmod_ctx
                .globals
                .get(global_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidGlobalIdx(global_idx.0))?;
            stack.pop(global_type.val_type)?;
        }
        // -- table instruction -- //
        TableGet { table_idx } => {
            let table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            stack.pop(t!(i32))?;
            stack.push(WasmValueType::Ref(table.ref_type));
        }
        TableSet { table_idx } => {
            let table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            let t = WasmValueType::Ref(table.ref_type);
            stack.pop(t)?;
            stack.pop(t!(i32))?;
            stack.push(t);
        }
        TableSize { table_idx } => {
            let _table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            stack.push(t!(i32));
        }
        TableGrow { table_idx } => {
            let table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            stack.pop(t!(i32))?;
            stack.pop(WasmValueType::Ref(table.ref_type))?;
            stack.push(t!(i32));
        }
        TableFill { table_idx } => {
            let table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            stack.pop(t!(i32))?;
            stack.pop(WasmValueType::Ref(table.ref_type))?;
            stack.pop(t!(i32))?;
        }
        TableCopy { src, dst } => {
            let src_table = wmod_ctx
                .tables
                .get(src.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(src.0))?;
            let dst_table = wmod_ctx
                .tables
                .get(dst.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(dst.0))?;
            if src_table.ref_type != dst_table.ref_type {
                return Err(WasmValidationError::MismatchedTableCopy {
                    src: src_table.ref_type,
                    dst: dst_table.ref_type,
                });
            }
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
        }
        TableInit {
            table_idx,
            elem_idx,
        } => {
            let table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            let elem = wmod_ctx
                .elements
                .get(elem_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidElemIdx(elem_idx.0))?;
            if table.ref_type != *elem {
                return Err(WasmValidationError::MismatchedTableInit {
                    table: table.ref_type,
                    elem: *elem,
                });
            }
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
        }
        ElemDrop { elem_idx } => {
            let _elem = wmod_ctx
                .elements
                .get(elem_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidElemIdx(elem_idx.0))?;
        }
        // -- memory instructions -- //
        I32Load { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i32), None)?;
        }
        I64Load { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i64), None)?;
        }
        F32Load { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(f32), None)?;
        }
        F64Load { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(f64), None)?;
        }
        I32Load8S { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i32), Some(8))?;
        }
        I32Load8U { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i32), Some(8))?;
        }
        I32Load16S { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i32), Some(16))?;
        }
        I32Load16U { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i32), Some(16))?;
        }
        I64Load8S { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i64), Some(8))?;
        }
        I64Load8U { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i64), Some(8))?;
        }
        I64Load16S { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i64), Some(16))?;
        }
        I64Load16U { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i64), Some(16))?;
        }
        I64Load32S { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i64), Some(32))?;
        }
        I64Load32U { memarg } => {
            validate_load_instr(wmod_ctx, memarg, stack, t!(i64), Some(32))?;
        }
        I32Store { memarg } => {
            validate_store_instr(wmod_ctx, memarg, stack, t!(i32), None)?;
        }
        I64Store { memarg } => {
            validate_store_instr(wmod_ctx, memarg, stack, t!(i64), None)?;
        }
        F32Store { memarg } => {
            validate_store_instr(wmod_ctx, memarg, stack, t!(f32), None)?;
        }
        F64Store { memarg } => {
            validate_store_instr(wmod_ctx, memarg, stack, t!(f64), None)?;
        }
        I32Store8 { memarg } => {
            validate_store_instr(wmod_ctx, memarg, stack, t!(i32), Some(8))?;
        }
        I32Store16 { memarg } => {
            validate_store_instr(wmod_ctx, memarg, stack, t!(i32), Some(16))?;
        }
        I64Store8 { memarg } => {
            validate_store_instr(wmod_ctx, memarg, stack, t!(i64), Some(8))?;
        }
        I64Store16 { memarg } => {
            validate_store_instr(wmod_ctx, memarg, stack, t!(i64), Some(16))?;
        }
        I64Store32 { memarg } => {
            validate_store_instr(wmod_ctx, memarg, stack, t!(i64), Some(32))?;
        }
        MemorySize => {
            let _mem = wmod_ctx
                .memories
                .get(0)
                .ok_or(WasmValidationError::NoMemory)?;
            stack.push(t!(i32));
        }
        MemoryGrow => {
            let _mem = wmod_ctx
                .memories
                .get(0)
                .ok_or(WasmValidationError::NoMemory)?;
            stack.pop(t!(i32))?;
            stack.push(t!(i32));
        }
        MemoryFill => {
            let _mem = wmod_ctx
                .memories
                .get(0)
                .ok_or(WasmValidationError::NoMemory)?;
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
        }
        MemoryCopy => {
            let _mem = wmod_ctx
                .memories
                .get(0)
                .ok_or(WasmValidationError::NoMemory)?;
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
        }
        MemoryInit { data_idx } => {
            let _mem = wmod_ctx
                .memories
                .get(0)
                .ok_or(WasmValidationError::NoMemory)?;
            if data_idx.0 as usize >= wmod_ctx.datas {
                return Err(WasmValidationError::InvalidDataIdx(data_idx.0));
            }
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
        }
        DataDrop { data_idx } => {
            if data_idx.0 as usize >= wmod_ctx.datas {
                return Err(WasmValidationError::InvalidDataIdx(data_idx.0));
            }
        }
        // -- control instructions -- //
        Nop => {}
        Unreachable => {}
        Block { block_type, expr } => {
            let func_type = validate_block_type(block_type, wmod_ctx)?;
            expr_ctx.labels.insert(0, func_type.output_type.clone());
            validate_instr_sequence_with_type(&expr.0, &func_type, wmod_ctx, expr_ctx)?;
            expr_ctx.labels.remove(0);
        }
        Loop { block_type, expr } => {
            let func_type = validate_block_type(block_type, wmod_ctx)?;
            expr_ctx.labels.insert(0, func_type.input_type.clone());
            validate_instr_sequence_with_type(&expr.0, &func_type, wmod_ctx, expr_ctx)?;
            expr_ctx.labels.remove(0);
        }
        If {
            block_type,
            then,
            else_,
        } => {
            let func_type = validate_block_type(block_type, wmod_ctx)?;
            expr_ctx.labels.insert(0, func_type.output_type.clone());
            validate_instr_sequence_with_type(&then.0, &func_type, wmod_ctx, expr_ctx)?;
            if let Some(else_) = else_ {
                validate_instr_sequence_with_type(&else_.0, &func_type, wmod_ctx, expr_ctx)?;
            }
            expr_ctx.labels.remove(0);
        }
        Break { label_idx } => {
            let result_type = expr_ctx
                .labels
                .get(label_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidLabelIdx(label_idx.0))?;
            stack.pop_result_type(&result_type)?;
        }
        BreakIf { label_idx } => {
            let result_type = expr_ctx
                .labels
                .get(label_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidLabelIdx(label_idx.0))?;
            stack.pop(t!(i32))?;
            stack.pop_result_type(&result_type)?;
        }
        BreakTable {
            labels: _,
            default_label: _,
        } => {
            todo!();
        }
        Return => match expr_ctx.ret {
            None => return Err(WasmValidationError::InvalidReturn),
            Some(ref result_type) => {
                stack.pop_result_type(result_type)?;
            }
        },
        Call { func_idx } => {
            let func_type = wmod_ctx
                .funcs
                .get(func_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidFuncIdx(func_idx.0))?;
            stack.pop_result_type(&func_type.input_type)?;
            stack.push_result_type(&func_type.output_type);
        }
        CallIndirect {
            table_idx,
            type_idx,
        } => {
            let table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            if table.ref_type != WasmRefType::FuncRef {
                return Err(WasmValidationError::InvalidCallIndirect);
            }
            let func_type = wmod_ctx
                .types
                .get(type_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTypeIdx(type_idx.0))?;
            stack.pop(t!(i32))?;
            stack.pop_result_type(&func_type.input_type)?;
            stack.push_result_type(&func_type.output_type);
        }
        Else => {}
        ExprEnd => {}
    }
    Ok(())
}

fn validate_instr_sequence_with_type(
    instrs: &[WasmInstruction],
    func_type: &WasmFuncType,
    wmod_ctx: &ModuleContext,
    expr_ctx: &mut ExprContext,
) -> WasmValidationResult<()> {
    let mut stack = TypeStack::from_input_type(&func_type.input_type);
    for op in instrs {
        validate_instr(op, wmod_ctx, expr_ctx, &mut stack)?;
    }
    stack.pop_result_type(&func_type.output_type)?;
    Ok(())
}

fn validate_expr_with_result_type(
    expr: &WasmExpr,
    result_type: &WasmResultType,
    wmod_ctx: &ModuleContext,
    expr_ctx: &mut ExprContext,
) -> WasmValidationResult<()> {
    let func_type = WasmFuncType {
        input_type: WasmResultType(Box::new([])),
        output_type: result_type.clone(),
    };
    validate_instr_sequence_with_type(&expr.0, &func_type, wmod_ctx, expr_ctx)?;
    Ok(())
}

fn validate_global_is_const(
    global_idx: WasmGlobalIdx,
    wmod_ctx: &ModuleContext,
) -> WasmValidationResult<()> {
    let global_type = wmod_ctx
        .globals
        .get(global_idx.0 as usize)
        .ok_or(WasmValidationError::InvalidGlobalIdx(global_idx.0))?;
    if global_type.mutability == WasmGlobalMutability::Immutable {
        Ok(())
    } else {
        Err(WasmValidationError::ExprNotConst)
    }
}

fn validate_expr_is_const(expr: &WasmExpr, wmod_ctx: &ModuleContext) -> WasmValidationResult<()> {
    for instr in &expr.0 {
        use WasmInstruction::*;
        match instr {
            I32Const { val: _ } => Ok(()),
            I64Const { val: _ } => Ok(()),
            F32Const { val: _ } => Ok(()),
            F64Const { val: _ } => Ok(()),
            RefNull { ref_type: _ } => Ok(()),
            RefFunc { func_idx: _ } => Ok(()),
            GlobalGet { global_idx } => validate_global_is_const(*global_idx, wmod_ctx),
            ExprEnd => Ok(()),
            _ => Err(WasmValidationError::ExprNotConst),
        }?;
    }
    Ok(())
}

mod context {

    use std::collections::HashSet;

    use super::*;

    pub struct ModuleContext<'wmod> {
        pub types: &'wmod [WasmFuncType],
        pub funcs: Vec<&'wmod WasmFuncType>,
        pub tables: Vec<&'wmod WasmTableType>,
        pub memories: Vec<&'wmod WasmMemType>,
        pub globals: Vec<&'wmod WasmGlobalType>,
        pub elements: Vec<WasmRefType>,
        pub datas: usize,
        pub refs: HashSet<WasmFuncIdx>,
    }

    pub struct ExprContext {
        pub locals: Vec<WasmValueType>,
        pub labels: Vec<WasmResultType>,
        pub ret: Option<WasmResultType>,
    }

    impl Default for ExprContext {
        fn default() -> Self {
            ExprContext {
                locals: vec![],
                labels: vec![],
                ret: None,
            }
        }
    }

    impl ExprContext {
        pub fn from_func<'wmod>(
            wmod_ctx: &ModuleContext<'wmod>,
            wfunc: &'wmod WasmFunc,
        ) -> WasmValidationResult<(WasmResultType, Self)> {
            let func_type = wmod_ctx
                .types
                .get(wfunc.type_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTypeIdx(wfunc.type_idx.0))?;
            let mut locals = vec![];
            locals.extend(func_type.input_type.0.iter());
            locals.extend(wfunc.locals.iter());
            Ok((
                func_type.output_type.clone(),
                ExprContext {
                    locals,
                    labels: vec![func_type.output_type.clone()],
                    ret: Some(func_type.output_type.clone()),
                },
            ))
        }

        pub fn with_return_type(val_type: WasmValueType) -> Self {
            let return_type = WasmResultType(vec![val_type].into_boxed_slice());
            ExprContext {
                locals: vec![],
                labels: vec![return_type.clone()],
                ret: Some(return_type),
            }
        }
    }

    impl<'wmod> ModuleContext<'wmod> {
        pub fn from_module(wmod: &'wmod WasmModule) -> WasmValidationResult<Self> {
            Ok(ModuleContext {
                types: context_types(wmod),
                funcs: context_funcs(wmod),
                tables: context_tables(wmod),
                memories: context_mems(wmod),
                globals: context_globals(wmod),
                elements: context_elems(wmod),
                datas: context_datas(wmod),
                refs: context_refs(wmod),
            })
        }

        pub fn include_internal_globals(&mut self, wmod: &'wmod WasmModule) {
            self.globals
                .extend(wmod.globals.iter().map(|g| &g.global_type))
        }
    }

    fn context_types(wmod: &WasmModule) -> &[WasmFuncType] {
        wmod.types.as_ref()
    }

    fn context_funcs(wmod: &WasmModule) -> Vec<&WasmFuncType> {
        let mut funcs = Vec::new();
        funcs.extend(wmod.imports.iter().filter_map(|i| match i.desc {
            WasmImportDesc::Func(ref f) => {
                Some(&wmod.types[wmod.funcs[f.0 as usize].type_idx.0 as usize])
            }
            _ => None,
        }));
        funcs.extend(
            wmod.funcs
                .iter()
                .map(|f| &wmod.types[f.type_idx.0 as usize]),
        );
        funcs
    }

    fn context_tables(wmod: &WasmModule) -> Vec<&WasmTableType> {
        let mut tables = Vec::new();
        tables.extend(wmod.imports.iter().filter_map(|i| match i.desc {
            WasmImportDesc::Table(ref t) => Some(t),
            _ => None,
        }));
        tables.extend(wmod.tables.iter());
        tables
    }

    fn context_mems(wmod: &WasmModule) -> Vec<&WasmMemType> {
        let mut mems = Vec::new();
        mems.extend(wmod.imports.iter().filter_map(|i| match i.desc {
            WasmImportDesc::Mem(ref m) => Some(m),
            _ => None,
        }));
        mems.extend(wmod.mems.iter());
        mems
    }

    fn context_globals(wmod: &WasmModule) -> Vec<&WasmGlobalType> {
        let mut globals = Vec::new();
        globals.extend(wmod.imports.iter().filter_map(|i| match i.desc {
            WasmImportDesc::Global(ref g) => Some(g),
            _ => None,
        }));
        globals
    }

    fn context_elems(wmod: &WasmModule) -> Vec<WasmRefType> {
        wmod.elems.iter().map(|e| e.ref_type).collect()
    }

    fn context_datas(wmod: &WasmModule) -> usize {
        wmod.datas.len()
    }

    fn context_refs(wmod: &WasmModule) -> HashSet<WasmFuncIdx> {
        let mut refs = HashSet::new();
        for data in &wmod.datas {
            if let WasmDataMode::Active {
                ref offset_expr, ..
            } = data.mode
            {
                add_const_expr_refs(offset_expr, &mut refs);
            }
        }
        for elem in &wmod.elems {
            if let WasmElemMode::Active {
                ref offset_expr, ..
            } = elem.elem_mode
            {
                add_const_expr_refs(offset_expr, &mut refs);
            }
        }
        for export in &wmod.exports {
            if let WasmExportDesc::Func(func_idx) = export.desc {
                refs.insert(func_idx);
            }
        }
        for global in &wmod.globals {
            add_const_expr_refs(&global.init, &mut refs);
        }
        refs
    }

    fn add_const_expr_refs(expr: &WasmExpr, refs: &mut HashSet<WasmFuncIdx>) {
        for op in &expr.0 {
            if let WasmInstruction::RefFunc { func_idx } = op {
                refs.insert(*func_idx);
            }
        }
    }
}

pub struct TypeStack(Vec<WasmValueType>);

impl TypeStack {
    pub fn from_input_type(input_type: &WasmResultType) -> Self {
        let mut vec = Vec::with_capacity(input_type.0.len());
        for t in &input_type.0 {
            vec.push(*t);
        }
        TypeStack(vec)
    }

    pub fn push(&mut self, t: WasmValueType) {
        self.0.push(t)
    }

    pub fn push_result_type(&mut self, result_type: &WasmResultType) {
        for t in &result_type.0 {
            self.push(*t);
        }
    }

    pub fn pop_any(&mut self) -> WasmValidationResult<WasmValueType> {
        self.0.pop().ok_or(WasmValidationError::MismatchedType {
            actual: None,
            // TODO: should be any type
            expected: WasmValueType::Num(WasmNumType::I32),
        })
    }

    pub fn pop_num_or_vec(&mut self) -> WasmValidationResult<WasmValueType> {
        match self.0.pop() {
            Some(t @ WasmValueType::Num(_)) => Ok(t),
            Some(t @ WasmValueType::Vec(_)) => Ok(t),
            actual => Err(WasmValidationError::MismatchedType {
                actual,
                // TODO: should be any type
                expected: WasmValueType::Num(WasmNumType::I32),
            }),
        }
    }

    pub fn pop(&mut self, expected: WasmValueType) -> WasmValidationResult<()> {
        match self.0.pop() {
            Some(t) if t == expected => Ok(()),
            actual => Err(WasmValidationError::MismatchedType { expected, actual }),
        }
    }

    pub fn pop_ref_type(&mut self) -> WasmValidationResult<()> {
        match self.0.pop() {
            Some(WasmValueType::Ref(_)) => Ok(()),
            actual => Err(WasmValidationError::MismatchedType {
                expected: WasmValueType::Ref(WasmRefType::FuncRef), // TODO represent this type better (any ref type)
                actual,
            }),
        }
    }

    pub fn pop_result_type(&mut self, result_type: &WasmResultType) -> WasmValidationResult<()> {
        for t in &result_type.0 {
            self.pop(*t)?;
        }
        Ok(())
    }
}
