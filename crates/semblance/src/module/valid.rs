use std::{cell::RefCell, collections::HashMap, rc::Rc};

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
        actual: Option<MaybeUntyped>,
    },
    MismatchedTableCopy {
        src: WasmRefType,
        dst: WasmRefType,
    },
    MismatchedTableInit {
        active: bool,
        table: WasmRefType,
        elem: WasmRefType,
    },
    TooManySelectTypes,
    InvalidReturn,
    InvalidCallIndirect,
    UnopenedBlock,
    InvalidElse,
    MissingRequiredElseBlock,
    UnexpectedStackDepth {
        expected: usize,
        actual: usize,
    },
    InvalidBlockArity {
        expected: usize,
        actual: usize,
    },
}

pub type WasmValidationResult<T> = Result<T, WasmValidationError>;

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum MaybeUntyped {
    UnknownType,
    KnownType(WasmValueType),
}

use context::*;

pub fn validate(wmod: WasmModuleRaw) -> WasmValidationResult<WasmModule> {
    // C'
    let mut wmod_ctx = ModuleContext::from_module(&wmod)?;
    for table in &wmod.tables {
        validate_table(table)?;
    }
    for mem in &wmod.mems {
        validate_mem(mem)?;
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
    wmod_ctx.include_internal_globals(&wmod);
    let mut side_tables = Vec::with_capacity(wmod.funcs.len());
    for func in &wmod.funcs {
        let side_table = validate_func(func, &wmod_ctx)?;
        side_tables.push(side_table);
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

    Ok(reencode_module_with_side_tables(wmod, side_tables))
}

type ControlFlowMap = HashMap<WasmInstructionIdx, WasmInstructionIdx>;
type BreakImmediatesMap = HashMap<WasmInstructionIdx, VerifiedBreakImmediates>;

pub struct ValidationSideTables {
    pub end_control_flow: ControlFlowMap,
    pub else_control_flow: ControlFlowMap,
    pub break_immediates: BreakImmediatesMap,
}

impl ValidationSideTables {
    pub fn new() -> Self {
        ValidationSideTables {
            end_control_flow: HashMap::new(),
            else_control_flow: HashMap::new(),
            break_immediates: HashMap::new(),
        }
    }
}

fn reencode_module_with_side_tables(
    wmod: WasmModuleRaw,
    side_tables: Vec<ValidationSideTables>,
) -> WasmModule {
    WasmModule {
        version: wmod.version,
        types: wmod.types,
        funcs: reencode_funcs_with_side_tables(wmod.funcs, side_tables),
        tables: wmod.tables,
        mems: wmod.mems,
        globals: wmod
            .globals
            .into_iter()
            .map(|global| WasmGlobal {
                global_type: global.global_type,
                init: reencode_const_expr(global.init),
            })
            .collect(),
        elems: wmod
            .elems
            .into_iter()
            .map(|elem| WasmElem {
                ref_type: elem.ref_type,
                init: elem.init.into_iter().map(reencode_const_expr).collect(),
                elem_mode: unsafe { std::mem::transmute(elem.elem_mode) },
            })
            .collect(),
        datas: wmod
            .datas
            .into_iter()
            .map(|data| WasmData {
                bytes: data.bytes,
                mode: unsafe { std::mem::transmute(data.mode) },
            })
            .collect(),
        start: wmod.start,
        imports: wmod.imports,
        exports: wmod.exports,
        customs: wmod.customs,
    }
}

fn reencode_const_expr(expr: Box<WasmExprRaw>) -> Box<WasmExpr> {
    expr.into_iter()
        .map(|instr| unsafe { std::mem::transmute(instr) })
        .collect()
}

fn reencode_funcs_with_side_tables(
    funcs: Box<[WasmFunc<WasmInstructionRaw>]>,
    side_tables: Vec<ValidationSideTables>,
) -> Box<[WasmFunc]> {
    funcs
        .into_iter()
        .zip(side_tables)
        .map(|(func, side_table)| reencode_func_with_side_tables(func, side_table))
        .collect()
}

fn reencode_func_with_side_tables(
    func: WasmFunc<WasmInstructionRaw>,
    side_tables: ValidationSideTables,
) -> WasmFunc {
    WasmFunc {
        type_idx: func.type_idx,
        locals: func.locals,
        body: reencode_expr_with_side_tables(func.body, side_tables),
    }
}

fn reencode_expr_with_side_tables(
    expr: Box<WasmExprRaw>,
    mut side_tables: ValidationSideTables,
) -> Box<WasmExpr> {
    expr.into_iter()
        .enumerate()
        .map(|(i, instr)| {
            let idx = WasmInstructionIdx(i as u32);
            reencode_instr_with_side_tables(
                instr,
                idx,
                side_tables.end_control_flow.remove(&idx),
                side_tables.else_control_flow.remove(&idx),
                side_tables.break_immediates.remove(&idx),
            )
        })
        .collect()
}

fn reencode_instr_with_side_tables(
    instr: WasmInstructionRaw,
    ic: WasmInstructionIdx,
    end_ic: Option<WasmInstructionIdx>,
    else_ic: Option<WasmInstructionIdx>,
    break_immediates: Option<VerifiedBreakImmediates>,
) -> WasmInstruction {
    use WasmInstructionRepr::*;
    match instr {
        If { block_type, imm: _ } => If {
            block_type,
            imm: VerifiedIfImmediates {
                end_off: calculate_relative_jump_offset(
                    ic,
                    end_ic.expect("missing control flow mapping"),
                ),
                else_off: else_ic.map(|else_ic| calculate_relative_jump_offset(ic, else_ic)),
            },
        },
        Block { block_type, imm: _ } => Block {
            block_type,
            imm: calculate_relative_jump_offset(ic, end_ic.expect("missing control flow mapping")),
        },
        Loop { block_type, imm: _ } => Loop {
            block_type,
            imm: calculate_relative_jump_offset(ic, end_ic.expect("missing control flow mapping")),
        },
        Break { label_idx, imm: _ } => Break {
            label_idx,
            imm: break_immediates.expect("missing break immediates"),
        },
        BreakIf { label_idx, imm: _ } => BreakIf {
            label_idx,
            imm: break_immediates.expect("missing break immediates"),
        },
        BreakTable { labels, imm: _ } => BreakTable {
            labels,
            imm: break_immediates.expect("missing break immediates"),
        },
        Return { imm: _ } => Return {
            imm: break_immediates.expect("missing break immediates"),
        },
        i @ _ => unsafe { std::mem::transmute(i) },
    }
}

fn calculate_relative_jump_offset(
    src: WasmInstructionIdx,
    dst: WasmInstructionIdx,
) -> WasmRelativeJumpOffset {
    debug_assert!(src.0 < dst.0);
    WasmRelativeJumpOffset(dst.0 - src.0)
}

fn validate_export_names(wmod: &WasmModuleRaw) -> WasmValidationResult<()> {
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
            validate_table(table_type)?;
            Ok(())
        }
        WasmImportDesc::Mem(ref mem_type) => {
            validate_mem(mem_type)?;
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

fn validate_func(
    func: &WasmFunc<WasmInstructionRaw>,
    wmod_ctx: &ModuleContext,
) -> WasmValidationResult<ValidationSideTables> {
    let expr_ctx = ExprContext::for_func(&wmod_ctx, func)?;
    validate_instr_sequence(&func.body, &wmod_ctx, expr_ctx)
}

fn validate_data(
    data: &WasmData<WasmInstructionRaw>,
    wmod_ctx: &ModuleContext,
) -> WasmValidationResult<()> {
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
    offset_expr: &WasmExprRaw,
    wmod_ctx: &ModuleContext,
) -> WasmValidationResult<()> {
    let _mem = wmod_ctx
        .memories
        .get(mem_idx.0 as usize)
        .ok_or(WasmValidationError::InvalidMemIdx(mem_idx.0))?;
    validate_instr_sequence(
        offset_expr,
        wmod_ctx,
        ExprContext::for_expr_of_type(t!(i32)),
    )?;
    validate_expr_is_const(offset_expr, wmod_ctx)
}

fn validate_elem(
    elem: &WasmElem<WasmInstructionRaw>,
    wmod_ctx: &ModuleContext,
) -> WasmValidationResult<()> {
    for expr in &elem.init {
        validate_instr_sequence(
            expr,
            wmod_ctx,
            ExprContext::for_expr_of_type(WasmValueType::Ref(elem.ref_type)),
        )?;
        validate_expr_is_const(expr, wmod_ctx)?;
    }
    match elem.elem_mode {
        WasmElemMode::Active {
            table_idx,
            ref offset_expr,
        } => validate_active_elem(elem.ref_type, table_idx, offset_expr, wmod_ctx),
        _ => Ok(()),
    }?;
    Ok(())
}

fn validate_active_elem(
    ref_type: WasmRefType,
    table_idx: WasmTableIdx,
    offset_expr: &WasmExprRaw,
    wmod_ctx: &ModuleContext,
) -> WasmValidationResult<()> {
    let table = wmod_ctx
        .tables
        .get(table_idx.0 as usize)
        .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
    if table.ref_type != ref_type {
        return Err(WasmValidationError::MismatchedTableInit {
            active: true,
            table: table.ref_type,
            elem: ref_type,
        });
    }
    validate_instr_sequence(
        offset_expr,
        wmod_ctx,
        ExprContext::for_expr_of_type(t!(i32)),
    )?;
    validate_expr_is_const(offset_expr, wmod_ctx)
}

fn validate_global(
    global: &WasmGlobal<WasmInstructionRaw>,
    wmod_ctx: &ModuleContext,
) -> WasmValidationResult<()> {
    let expr_context = ExprContext::for_expr_of_type(global.global_type.val_type);
    validate_instr_sequence(&global.init, wmod_ctx, expr_context)?;
    validate_expr_is_const(&global.init, wmod_ctx)
}

const MEM_RANGE: u32 = (2 as u32).pow(16);

fn validate_mem(mem: &WasmMemType) -> WasmValidationResult<()> {
    validate_limits_within_range(&mem.limits, MEM_RANGE)
}

const TABLE_RANGE: u32 = u32::MAX;

fn validate_table(table: &WasmTableType) -> WasmValidationResult<()> {
    validate_limits_within_range(&table.limits, TABLE_RANGE)
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
    expr_ctx: &mut ExprContext,
    memarg: &WasmMemArg,
    t: WasmValueType,
    bits: Option<u32>,
) -> WasmValidationResult<()> {
    let _mem = wmod_ctx
        .memories
        .get(0)
        .ok_or(WasmValidationError::NoMemory)?;
    validate_alignment(memarg, t, bits)?;
    let stack = expr_ctx.stack();
    stack.pop(t!(i32))?;
    stack.push(t);
    Ok(())
}

fn validate_store_instr(
    wmod_ctx: &ModuleContext,
    expr_ctx: &mut ExprContext,
    memarg: &WasmMemArg,
    t: WasmValueType,
    bits: Option<u32>,
) -> WasmValidationResult<()> {
    let _mem = wmod_ctx
        .memories
        .get(0)
        .ok_or(WasmValidationError::NoMemory)?;
    validate_alignment(memarg, t, bits)?;
    let stack = expr_ctx.stack();
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
    op: &WasmInstructionRaw,
    wmod_ctx: &ModuleContext,
    expr_ctx: &mut ExprContext,
    idx: WasmInstructionIdx,
) -> WasmValidationResult<()> {
    use WasmInstructionRepr::*;
    match op {
        // -- t.const -- //
        I32Const { .. } => {
            expr_ctx.stack().push(t!(i32));
        }
        I64Const { .. } => {
            expr_ctx.stack().push(t!(i64));
        }
        F32Const { .. } => {
            expr_ctx.stack().push(t!(f32));
        }
        F64Const { .. } => {
            expr_ctx.stack().push(t!(f64));
        }
        // -- t.unop -- //
        I32Clz | I32Ctz | I32Popcnt | I32Extend8S | I32Extend16S => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.push(t!(i32));
        }
        I64Clz | I64Ctz | I64Popcnt | I64Extend8S | I64Extend16S | I64Extend32S => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i64))?;
            stack.push(t!(i64));
        }
        F32Abs | F32Neg | F32Sqrt | F32Ceil | F32Floor | F32Trunc | F32Nearest => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f32))?;
            stack.push(t!(f32));
        }
        F64Abs | F64Neg | F64Sqrt | F64Ceil | F64Floor | F64Trunc | F64Nearest => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f64))?;
            stack.push(t!(f64));
        }
        // -- t.binop -- //
        I32Add | I32Sub | I32Mul | I32DivS | I32DivU | I32RemS | I32RemU | I32And | I32Or
        | I32Xor | I32Shl | I32ShrS | I32ShrU | I32Rotl | I32Rotr => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
            stack.push(t!(i32));
        }
        I64Add | I64Sub | I64Mul | I64DivS | I64DivU | I64RemS | I64RemU | I64And | I64Or
        | I64Xor | I64Shl | I64ShrS | I64ShrU | I64Rotl | I64Rotr => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i64))?;
            stack.pop(t!(i64))?;
            stack.push(t!(i64));
        }
        F32Add | F32Sub | F32Mul | F32Div | F32Min | F32Max | F32CopySign => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f32))?;
            stack.pop(t!(f32))?;
            stack.push(t!(f32));
        }
        F64Add | F64Sub | F64Mul | F64Div | F64Min | F64Max | F64CopySign => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f64))?;
            stack.pop(t!(f64))?;
            stack.push(t!(f64));
        }
        // -- t.testop -- //
        I32EqZ => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.push(t!(i32));
        }
        I64EqZ => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i64))?;
            stack.push(t!(i32));
        }
        // -- t.relop -- //
        I32Eq | I32Neq | I32LtS | I32LtU | I32GtS | I32GtU | I32LeS | I32LeU | I32GeS | I32GeU => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
            stack.push(t!(i32));
        }
        I64Eq | I64Neq | I64LtS | I64LtU | I64GtS | I64GtU | I64LeS | I64LeU | I64GeS | I64GeU => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i64))?;
            stack.pop(t!(i64))?;
            stack.push(t!(i32));
        }
        F32Eq | F32Neq | F32Lt | F32Gt | F32Le | F32Ge => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f32))?;
            stack.pop(t!(f32))?;
            stack.push(t!(i32));
        }
        F64Eq | F64Neq | F64Lt | F64Gt | F64Le | F64Ge => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f64))?;
            stack.pop(t!(f64))?;
            stack.push(t!(i32));
        }
        // -- t2.cvtop_t1_sx -- //
        I32WrapI64 => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i64))?;
            stack.push(t!(i32));
        }
        I32TruncF32S | I32TruncF32U | I32TruncSatF32S | I32TruncSatF32U | I32ReinterpretF32 => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f32))?;
            stack.push(t!(i32));
        }
        I32TruncF64S | I32TruncF64U | I32TruncSatF64S | I32TruncSatF64U => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f64))?;
            stack.push(t!(i32));
        }
        I64ExtendI32S | I64ExtendI32U => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.push(t!(i64));
        }
        I64TruncF32S | I64TruncF32U | I64TruncSatF32S | I64TruncSatF32U => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f32))?;
            stack.push(t!(i64));
        }
        I64TruncF64S | I64TruncF64U | I64TruncSatF64S | I64TruncSatF64U | I64ReinterpretF64 => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f64))?;
            stack.push(t!(i64));
        }
        F32ConvertI32S | F32ConvertI32U | F32ReinterpretI32 => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.push(t!(f32));
        }
        F32ConvertI64S | F32ConvertI64U => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i64))?;
            stack.push(t!(f32));
        }
        F32DemoteF64 => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f64))?;
            stack.push(t!(f32));
        }
        F64ConvertI32S | F64ConvertI32U => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.push(t!(f64));
        }
        F64ConvertI64S | F64ConvertI64U | F64ReinterpretI64 => {
            let stack = expr_ctx.stack();
            stack.pop(t!(i64))?;
            stack.push(t!(f64));
        }
        F64PromoteF32 => {
            let stack = expr_ctx.stack();
            stack.pop(t!(f32))?;
            stack.push(t!(f64));
        }
        // -- reference instructions -- //
        RefNull { ref_type } => {
            expr_ctx.stack().push(WasmValueType::Ref(*ref_type));
        }
        RefIsNull => {
            let stack = expr_ctx.stack();
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
            expr_ctx.stack().push(t!(funcref));
        }
        // -- parametric instructions -- //
        Drop => {
            expr_ctx.stack().pop_any()?;
        }
        Select { value_types } => match value_types.len() {
            0 => {
                let stack = expr_ctx.stack();
                stack.pop(t!(i32))?;
                let t = stack.pop_num_or_vec()?;
                stack.pop_dyn(t)?;
                stack.push_dyn(t);
            }
            1 => {
                let t = value_types[0];
                let stack = expr_ctx.stack();
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
            expr_ctx.stack().push(*local_type);
        }
        LocalSet { local_idx } => {
            let local_type = expr_ctx
                .locals
                .get(local_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidLocalIdx(local_idx.0))?;
            expr_ctx.stack().pop(*local_type)?;
        }
        LocalTee { local_idx } => {
            let local_type = *expr_ctx
                .locals
                .get(local_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidLocalIdx(local_idx.0))?;
            let stack = expr_ctx.stack();
            stack.pop(local_type)?;
            stack.push(local_type);
        }
        GlobalGet { global_idx } => {
            let global_type = wmod_ctx
                .globals
                .get(global_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidGlobalIdx(global_idx.0))?;
            expr_ctx.stack().push(global_type.val_type);
        }
        GlobalSet { global_idx } => {
            let global_type = wmod_ctx
                .globals
                .get(global_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidGlobalIdx(global_idx.0))?;
            expr_ctx.stack().pop(global_type.val_type)?;
        }
        // -- table instruction -- //
        TableGet { table_idx } => {
            let table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.push(WasmValueType::Ref(table.ref_type));
        }
        TableSet { table_idx } => {
            let table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            let t = WasmValueType::Ref(table.ref_type);
            let stack = expr_ctx.stack();
            stack.pop(t)?;
            stack.pop(t!(i32))?;
        }
        TableSize { table_idx } => {
            let _table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            expr_ctx.stack().push(t!(i32));
        }
        TableGrow { table_idx } => {
            let table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.pop(WasmValueType::Ref(table.ref_type))?;
            stack.push(t!(i32));
        }
        TableFill { table_idx } => {
            let table = wmod_ctx
                .tables
                .get(table_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTableIdx(table_idx.0))?;
            let stack = expr_ctx.stack();
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
            let stack = expr_ctx.stack();
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
                    active: false,
                    table: table.ref_type,
                    elem: *elem,
                });
            }
            let stack = expr_ctx.stack();
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
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i32), None)?;
        }
        I64Load { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i64), None)?;
        }
        F32Load { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(f32), None)?;
        }
        F64Load { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(f64), None)?;
        }
        I32Load8S { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i32), Some(8))?;
        }
        I32Load8U { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i32), Some(8))?;
        }
        I32Load16S { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i32), Some(16))?;
        }
        I32Load16U { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i32), Some(16))?;
        }
        I64Load8S { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i64), Some(8))?;
        }
        I64Load8U { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i64), Some(8))?;
        }
        I64Load16S { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i64), Some(16))?;
        }
        I64Load16U { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i64), Some(16))?;
        }
        I64Load32S { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i64), Some(32))?;
        }
        I64Load32U { memarg } => {
            validate_load_instr(wmod_ctx, expr_ctx, memarg, t!(i64), Some(32))?;
        }
        I32Store { memarg } => {
            validate_store_instr(wmod_ctx, expr_ctx, memarg, t!(i32), None)?;
        }
        I64Store { memarg } => {
            validate_store_instr(wmod_ctx, expr_ctx, memarg, t!(i64), None)?;
        }
        F32Store { memarg } => {
            validate_store_instr(wmod_ctx, expr_ctx, memarg, t!(f32), None)?;
        }
        F64Store { memarg } => {
            validate_store_instr(wmod_ctx, expr_ctx, memarg, t!(f64), None)?;
        }
        I32Store8 { memarg } => {
            validate_store_instr(wmod_ctx, expr_ctx, memarg, t!(i32), Some(8))?;
        }
        I32Store16 { memarg } => {
            validate_store_instr(wmod_ctx, expr_ctx, memarg, t!(i32), Some(16))?;
        }
        I64Store8 { memarg } => {
            validate_store_instr(wmod_ctx, expr_ctx, memarg, t!(i64), Some(8))?;
        }
        I64Store16 { memarg } => {
            validate_store_instr(wmod_ctx, expr_ctx, memarg, t!(i64), Some(16))?;
        }
        I64Store32 { memarg } => {
            validate_store_instr(wmod_ctx, expr_ctx, memarg, t!(i64), Some(32))?;
        }
        MemorySize => {
            let _mem = wmod_ctx
                .memories
                .get(0)
                .ok_or(WasmValidationError::NoMemory)?;
            expr_ctx.stack().push(t!(i32));
        }
        MemoryGrow => {
            let _mem = wmod_ctx
                .memories
                .get(0)
                .ok_or(WasmValidationError::NoMemory)?;
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.push(t!(i32));
        }
        MemoryFill => {
            let _mem = wmod_ctx
                .memories
                .get(0)
                .ok_or(WasmValidationError::NoMemory)?;
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
            stack.pop(t!(i32))?;
        }
        MemoryCopy => {
            let _mem = wmod_ctx
                .memories
                .get(0)
                .ok_or(WasmValidationError::NoMemory)?;
            let stack = expr_ctx.stack();
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
            let stack = expr_ctx.stack();
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
        Unreachable => {
            expr_ctx.unreachable();
        }
        Block { block_type, imm: _ } => {
            let func_type = validate_block_type(block_type, wmod_ctx)?;
            expr_ctx.stack().pop_result_type(&func_type.input_type)?;
            expr_ctx.push_label(LabelEntry {
                ty: func_type,
                idx: Some(idx),
                opcode: LabelOpcode::Block,
                min_stack_depth: expr_ctx.stack().depth(),
                unreachable: false,
            });
        }
        Loop { block_type, imm: _ } => {
            let func_type = validate_block_type(block_type, wmod_ctx)?;
            expr_ctx.stack().pop_result_type(&func_type.input_type)?;
            expr_ctx.push_label(LabelEntry {
                ty: func_type,
                idx: Some(idx),
                opcode: LabelOpcode::Loop,
                min_stack_depth: expr_ctx.stack().depth(),
                unreachable: false,
            });
        }
        If { block_type, imm: _ } => {
            let func_type = validate_block_type(block_type, wmod_ctx)?;
            expr_ctx.stack().pop(t!(i32))?;
            expr_ctx.stack().pop_result_type(&func_type.input_type)?;
            expr_ctx.push_label(LabelEntry {
                ty: func_type,
                idx: Some(idx),
                opcode: LabelOpcode::If,
                min_stack_depth: expr_ctx.stack().depth(),
                unreachable: false,
            });
        }
        Break { label_idx, imm: _ } => {
            let label_entry = expr_ctx
                .labels
                .peek(*label_idx)
                .ok_or(WasmValidationError::InvalidLabelIdx(label_idx.0))?;
            let ty = label_entry.label_types();
            let arity = ty.len().try_into().expect("arity immediate too large");
            expr_ctx.stack().pop_result_type(ty)?;
            let drop = expr_ctx
                .unreachable()
                .try_into()
                .expect("drop immediate too large");
            expr_ctx
                .side_tables
                .break_immediates
                .insert(idx, VerifiedBreakImmediates { arity, drop });
        }
        BreakIf { label_idx, imm: _ } => {
            let label_entry = expr_ctx
                .labels
                .peek(*label_idx)
                .ok_or(WasmValidationError::InvalidLabelIdx(label_idx.0))?;
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            let label_types = label_entry.label_types();
            let arity = label_types
                .len()
                .try_into()
                .expect("arity immediate too large");
            stack.pop_result_type(label_types)?;
            let drop = (stack.depth() - label_entry.min_stack_depth)
                .try_into()
                .expect("drop immediate too large");
            stack.push_result_type(label_types);
            expr_ctx
                .side_tables
                .break_immediates
                .insert(idx, VerifiedBreakImmediates { arity, drop });
        }
        BreakTable {
            labels: all_labels,
            imm: _,
        } => {
            let default_label = all_labels.last().unwrap();
            let labels = &all_labels[0..all_labels.len() - 1];
            let default_label_entry = expr_ctx
                .labels
                .peek(*default_label)
                .ok_or(WasmValidationError::InvalidLabelIdx(default_label.0))?;
            let ty = default_label_entry.label_types();
            let arity = ty.len();
            {
                let stack = expr_ctx.stack();
                stack.pop(t!(i32))?;
                for label_idx in labels {
                    let label_entry = expr_ctx
                        .labels
                        .peek(*label_idx)
                        .ok_or(WasmValidationError::InvalidLabelIdx(label_idx.0))?;
                    let types = label_entry.label_types();
                    if types.len() != arity {
                        return Err(WasmValidationError::InvalidBlockArity {
                            expected: arity,
                            actual: types.len(),
                        });
                    }
                    stack.pop_result_type(types)?;
                    stack.push_result_type(types);
                }
                stack.pop_result_type(ty)?;
            }
            let drop = expr_ctx.unreachable();
            expr_ctx.side_tables.break_immediates.insert(
                idx,
                VerifiedBreakImmediates {
                    arity: arity.try_into().expect("arity immediate too large"),
                    drop: drop.try_into().expect("drop immediate too large"),
                },
            );
        }
        Return { imm: _ } => match expr_ctx.ret {
            None => return Err(WasmValidationError::InvalidReturn),
            Some(ref result_type) => {
                expr_ctx.stack().pop_result_type(result_type)?;
                let arity = result_type.len();
                let drop = expr_ctx.unreachable();
                expr_ctx.side_tables.break_immediates.insert(
                    idx,
                    VerifiedBreakImmediates {
                        arity: arity.try_into().expect("arity immediate too large"),
                        drop: drop.try_into().expect("drop immediate too large"),
                    },
                );
            }
        },
        Call { func_idx } => {
            let func_type = wmod_ctx
                .funcs
                .get(func_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidFuncIdx(func_idx.0))?;
            let stack = expr_ctx.stack();
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
            let stack = expr_ctx.stack();
            stack.pop(t!(i32))?;
            stack.pop_result_type(&func_type.input_type)?;
            stack.push_result_type(&func_type.output_type);
        }
        Else => {
            let label_entry = expr_ctx.pop_label()?;
            if label_entry.opcode != LabelOpcode::If {
                return Err(WasmValidationError::InvalidElse);
            }
            expr_ctx.push_label(LabelEntry {
                ty: label_entry.ty,
                idx: label_entry.idx,
                opcode: LabelOpcode::Else,
                min_stack_depth: label_entry.min_stack_depth,
                unreachable: false,
            });
            expr_ctx
                .side_tables
                .else_control_flow
                .insert(label_entry.idx.unwrap(), idx);
        }
        ExprEnd => {
            let label_entry = expr_ctx.pop_label()?;
            if label_entry.opcode == LabelOpcode::If {
                // no else block, typecheck empty expression
                let stack = expr_ctx.stack();
                stack.push_result_type(&label_entry.ty.input_type);
                stack.pop_result_type(&label_entry.ty.output_type)?;
            }
            expr_ctx
                .stack()
                .push_result_type(&label_entry.ty.output_type);
            if let Some(start_idx) = label_entry.idx {
                expr_ctx.side_tables.end_control_flow.insert(start_idx, idx);
            }
        }
    }
    Ok(())
}

fn validate_instr_sequence(
    instrs: &WasmExprRaw,
    wmod_ctx: &ModuleContext,
    mut expr_ctx: ExprContext,
) -> WasmValidationResult<ValidationSideTables> {
    for (i, op) in instrs.iter().enumerate() {
        validate_instr(op, wmod_ctx, &mut expr_ctx, WasmInstructionIdx(i as u32))?;
    }
    Ok(expr_ctx.consume_side_tables())
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

fn validate_expr_is_const(
    expr: &WasmExprRaw,
    wmod_ctx: &ModuleContext,
) -> WasmValidationResult<()> {
    for instr in expr {
        use WasmInstructionRepr::*;
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

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub enum LabelOpcode {
        Block,
        If,
        Else,
        Loop,
    }

    pub struct LabelEntry {
        pub ty: WasmFuncType,
        pub idx: Option<WasmInstructionIdx>,
        pub opcode: LabelOpcode,
        pub min_stack_depth: usize,
        pub unreachable: bool,
    }

    impl LabelEntry {
        pub fn label_types(&self) -> &WasmResultType {
            if self.opcode == LabelOpcode::Loop {
                &self.ty.input_type
            } else {
                &self.ty.output_type
            }
        }
    }

    pub struct LabelStack(Vec<LabelEntry>);

    impl LabelStack {
        pub fn with_result_type(ty: WasmResultType) -> Self {
            let ty = WasmFuncType {
                input_type: WasmResultType(Box::new([])),
                output_type: ty,
            };
            LabelStack(vec![LabelEntry {
                ty,
                idx: None,
                opcode: LabelOpcode::Block,
                min_stack_depth: 0,
                unreachable: false,
            }])
        }

        pub fn with_func_type(ty: WasmFuncType) -> Self {
            LabelStack(vec![LabelEntry {
                ty,
                idx: None,
                opcode: LabelOpcode::Block,
                min_stack_depth: 0,
                unreachable: false,
            }])
        }

        pub fn push(&mut self, entry: LabelEntry) {
            self.0.push(entry);
        }

        pub fn pop(&mut self) -> Option<LabelEntry> {
            self.0.pop()
        }

        pub fn peek_top(&self) -> Option<&LabelEntry> {
            self.0.last()
        }

        pub fn peek_top_mut(&mut self) -> Option<&mut LabelEntry> {
            self.0.last_mut()
        }

        pub fn peek(&self, label_idx: WasmLabelIdx) -> Option<&LabelEntry> {
            if self.0.len() < label_idx.0 as usize + 1 {
                return None;
            }
            let idx = (self.0.len() - 1) - label_idx.0 as usize;
            self.0.get(idx)
        }
    }

    pub struct ExprContext {
        stack: TypeStack,
        pub locals: Vec<WasmValueType>,
        pub labels: LabelStack,
        pub ret: Option<WasmResultType>,
        pub side_tables: ValidationSideTables,
    }

    impl ExprContext {
        pub fn for_func<'wmod>(
            wmod_ctx: &ModuleContext<'wmod>,
            wfunc: &'wmod WasmFunc<WasmInstructionRaw>,
        ) -> WasmValidationResult<Self> {
            let func_type = wmod_ctx
                .types
                .get(wfunc.type_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTypeIdx(wfunc.type_idx.0))?;
            let mut locals = vec![];
            locals.extend(func_type.input_type.0.iter());
            locals.extend(wfunc.locals.iter());
            Ok(ExprContext {
                locals,
                stack: TypeStack::empty(),
                labels: LabelStack::with_func_type(func_type.clone()),
                ret: Some(func_type.output_type.clone()),
                side_tables: ValidationSideTables::new(),
            })
        }

        pub fn for_expr_of_type(val_type: WasmValueType) -> Self {
            let return_type = WasmResultType(vec![val_type].into_boxed_slice());
            ExprContext {
                stack: TypeStack::empty(),
                locals: vec![],
                labels: LabelStack::with_result_type(return_type.clone()),
                ret: None,
                side_tables: ValidationSideTables::new(),
            }
        }

        pub fn consume_side_tables(self) -> ValidationSideTables {
            self.side_tables
        }

        pub fn stack(&self) -> TypeStack {
            let (min_stack_depth, unreachable) = self
                .labels
                .0
                .last()
                .map(|l| (l.min_stack_depth, l.unreachable))
                .unwrap_or((0, false));
            self.stack.with_control_data(min_stack_depth, unreachable)
        }

        pub fn pop_label(&mut self) -> WasmValidationResult<LabelEntry> {
            let entry = self
                .labels
                .peek_top()
                .ok_or(WasmValidationError::UnopenedBlock)?;
            self.stack().pop_result_type(&entry.ty.output_type)?;
            if self.stack.depth() != entry.min_stack_depth {
                return Err(WasmValidationError::UnexpectedStackDepth {
                    expected: entry.min_stack_depth,
                    actual: self.stack.depth(),
                });
            }
            Ok(self.labels.pop().unwrap())
        }

        pub fn push_label(&mut self, label_entry: LabelEntry) {
            self.stack.push_result_type(&label_entry.ty.input_type);
            self.labels.push(label_entry);
        }

        pub fn unreachable(&mut self) -> usize {
            let label_entry = self.labels.peek_top_mut().expect("no label!");
            let dropped = self.stack.depth() - label_entry.min_stack_depth;
            self.stack
                .stack
                .borrow_mut()
                .truncate(label_entry.min_stack_depth);
            label_entry.unreachable = true;
            dropped
        }
    }

    impl<'wmod> ModuleContext<'wmod> {
        pub fn from_module(wmod: &'wmod WasmModuleRaw) -> WasmValidationResult<Self> {
            Ok(ModuleContext {
                types: context_types(wmod),
                funcs: context_funcs(wmod)?,
                tables: context_tables(wmod),
                memories: context_mems(wmod),
                globals: context_globals(wmod),
                elements: context_elems(wmod),
                datas: context_datas(wmod),
                refs: context_refs(wmod),
            })
        }

        pub fn include_internal_globals(&mut self, wmod: &'wmod WasmModuleRaw) {
            self.globals
                .extend(wmod.globals.iter().map(|g| &g.global_type))
        }
    }

    fn context_types(wmod: &WasmModuleRaw) -> &[WasmFuncType] {
        wmod.types.as_ref()
    }

    fn context_funcs(wmod: &WasmModuleRaw) -> WasmValidationResult<Vec<&WasmFuncType>> {
        let mut funcs = Vec::new();
        for import in &wmod.imports {
            if let WasmImportDesc::Func(ref typeidx) = import.desc {
                let ty = wmod
                    .types
                    .get(typeidx.0 as usize)
                    .ok_or(WasmValidationError::InvalidTypeIdx(typeidx.0))?;
                funcs.push(ty);
            }
        }
        for func in &wmod.funcs {
            let ty = wmod
                .types
                .get(func.type_idx.0 as usize)
                .ok_or(WasmValidationError::InvalidTypeIdx(func.type_idx.0))?;
            funcs.push(ty);
        }
        Ok(funcs)
    }

    fn context_tables(wmod: &WasmModuleRaw) -> Vec<&WasmTableType> {
        let mut tables = Vec::new();
        tables.extend(wmod.imports.iter().filter_map(|i| match i.desc {
            WasmImportDesc::Table(ref t) => Some(t),
            _ => None,
        }));
        tables.extend(wmod.tables.iter());
        tables
    }

    fn context_mems(wmod: &WasmModuleRaw) -> Vec<&WasmMemType> {
        let mut mems = Vec::new();
        mems.extend(wmod.imports.iter().filter_map(|i| match i.desc {
            WasmImportDesc::Mem(ref m) => Some(m),
            _ => None,
        }));
        mems.extend(wmod.mems.iter());
        mems
    }

    fn context_globals(wmod: &WasmModuleRaw) -> Vec<&WasmGlobalType> {
        let mut globals = Vec::new();
        globals.extend(wmod.imports.iter().filter_map(|i| match i.desc {
            WasmImportDesc::Global(ref g) => Some(g),
            _ => None,
        }));
        globals
    }

    fn context_elems(wmod: &WasmModuleRaw) -> Vec<WasmRefType> {
        wmod.elems.iter().map(|e| e.ref_type).collect()
    }

    fn context_datas(wmod: &WasmModuleRaw) -> usize {
        wmod.datas.len()
    }

    fn context_refs(wmod: &WasmModuleRaw) -> HashSet<WasmFuncIdx> {
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
            for init_expr in &elem.init {
                add_const_expr_refs(init_expr, &mut refs);
            }
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

    fn add_const_expr_refs(expr: &WasmExprRaw, refs: &mut HashSet<WasmFuncIdx>) {
        for op in expr {
            if let WasmInstructionRepr::RefFunc { func_idx } = op {
                refs.insert(*func_idx);
            }
        }
    }
}

pub struct TypeStack {
    stack: Rc<RefCell<Vec<MaybeUntyped>>>,
    min_depth: usize,
    unreachable: bool,
}

impl TypeStack {
    pub fn empty() -> Self {
        TypeStack {
            stack: Rc::new(RefCell::new(vec![])),
            min_depth: 0,
            unreachable: false,
        }
    }

    pub fn with_control_data(&self, min_depth: usize, unreachable: bool) -> TypeStack {
        TypeStack {
            stack: self.stack.clone(),
            min_depth,
            unreachable,
        }
    }

    pub fn push(&self, t: WasmValueType) {
        self.stack.borrow_mut().push(MaybeUntyped::KnownType(t))
    }

    pub fn push_dyn(&self, t: MaybeUntyped) {
        self.stack.borrow_mut().push(t)
    }

    pub fn push_result_type(&self, result_type: &WasmResultType) {
        for t in result_type.0.iter() {
            self.push(*t);
        }
    }

    fn pop_checked(&self) -> Option<MaybeUntyped> {
        if self.depth() > self.min_depth {
            self.stack.borrow_mut().pop()
        } else if self.unreachable {
            Some(MaybeUntyped::UnknownType)
        } else {
            None
        }
    }

    pub fn pop_any(&mut self) -> WasmValidationResult<()> {
        self.pop_checked()
            .ok_or(WasmValidationError::MismatchedType {
                actual: None,
                // TODO: should be any type
                expected: WasmValueType::Num(WasmNumType::I32),
            })?;
        Ok(())
    }

    pub fn pop_num_or_vec(&self) -> WasmValidationResult<MaybeUntyped> {
        match self.pop_checked() {
            Some(t @ MaybeUntyped::KnownType(WasmValueType::Num(_))) => Ok(t),
            Some(t @ MaybeUntyped::KnownType(WasmValueType::Vec(_))) => Ok(t),
            Some(t @ MaybeUntyped::UnknownType) => Ok(t),
            actual => Err(WasmValidationError::MismatchedType {
                actual,
                // TODO: should be num|vec
                expected: WasmValueType::Num(WasmNumType::I32),
            }),
        }
    }

    pub fn pop(&self, expected: WasmValueType) -> WasmValidationResult<()> {
        match self.pop_checked() {
            Some(MaybeUntyped::KnownType(t)) if t == expected => Ok(()),
            Some(MaybeUntyped::UnknownType) => Ok(()),
            actual => Err(WasmValidationError::MismatchedType { expected, actual }),
        }
    }

    pub fn pop_dyn(&self, expected: MaybeUntyped) -> WasmValidationResult<()> {
        match expected {
            MaybeUntyped::UnknownType => Ok(()),
            MaybeUntyped::KnownType(t) => self.pop(t),
        }
    }

    pub fn pop_ref_type(&self) -> WasmValidationResult<()> {
        match self.pop_checked() {
            Some(MaybeUntyped::KnownType(WasmValueType::Ref(_))) => Ok(()),
            Some(MaybeUntyped::UnknownType) => Ok(()),
            actual => Err(WasmValidationError::MismatchedType {
                expected: WasmValueType::Ref(WasmRefType::FuncRef), // TODO represent this type better (any ref type)
                actual,
            }),
        }
    }

    pub fn pop_result_type(&self, result_type: &WasmResultType) -> WasmValidationResult<()> {
        for t in result_type.0.iter().rev() {
            self.pop(*t)?;
        }
        Ok(())
    }

    pub fn depth(&self) -> usize {
        self.stack.borrow().len()
    }
}
