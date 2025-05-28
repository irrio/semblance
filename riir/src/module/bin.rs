use std::string::FromUtf8Error;

use super::{
    builder::{WasmCode, WasmExprBuilder, WasmModuleBuilder, WasmResultTypeBuilder},
    *,
};

#[derive(Debug)]
pub enum WasmDecodeError {
    MagicBytes,
    UnsupportedVersion(u32),
    UnknownSectionId(u8),
    NonUtfName(FromUtf8Error),
    InvalidFuncType(u8),
    InvalidValueType(u8),
    InvalidImportDesc(u8),
    InvalidRefType(u8),
    InvalidLimits(u8),
    InvalidGlobalMutability(u8),
    InvalidExportDesc(u8),
    UnknownOpcode(u8),
    UnknownExtendedOpcode(u32),
    UnexpectedEof,
}

pub type WasmDecodeResult<T> = Result<T, WasmDecodeError>;

type Decoded<'b, T> = (T, &'b [u8]);

fn take_bytes<const N: usize>(bytes: &[u8]) -> WasmDecodeResult<Decoded<[u8; N]>> {
    if bytes.len() < N {
        return Err(WasmDecodeError::UnexpectedEof);
    }
    let mut buf = [0u8; N];
    buf.copy_from_slice(&bytes[0..N]);
    Ok((buf, &bytes[N..]))
}

fn take_byte(bytes: &[u8]) -> WasmDecodeResult<Decoded<u8>> {
    let (buf, bytes) = take_bytes::<1>(bytes)?;
    Ok((buf[0], bytes))
}

fn take_bytes_dyn(bytes: &[u8], n: usize) -> WasmDecodeResult<Decoded<Vec<u8>>> {
    if bytes.len() < n {
        return Err(WasmDecodeError::UnexpectedEof);
    }
    let mut buf = Vec::with_capacity(n);
    buf.extend_from_slice(&bytes[0..n]);
    Ok((buf, &bytes[n..]))
}

fn decode_leb128(mut bytes: &[u8]) -> WasmDecodeResult<Decoded<u32>> {
    let mut result = 0;
    let mut shift = 0;
    loop {
        let (byte, rest) = take_byte(bytes)?;
        bytes = rest;
        result |= ((byte & !(1 << 7)) as u32) << shift;
        if byte & (1 << 7) == 0 {
            break;
        }
        shift += 7;
    }
    Ok((result, bytes))
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum SectionId {
    Custom = 0,
    Type = 1,
    Import = 2,
    Function = 3,
    Table = 4,
    Memory = 5,
    Global = 6,
    Export = 7,
    Start = 8,
    Element = 9,
    Code = 10,
    Data = 11,
    DataCount = 12,
}

fn decode_section_id(bytes: &[u8]) -> WasmDecodeResult<Decoded<SectionId>> {
    let (byte, bytes) = take_byte(bytes)?;
    let sid = match byte {
        0 => Ok(SectionId::Custom),
        1 => Ok(SectionId::Type),
        2 => Ok(SectionId::Import),
        3 => Ok(SectionId::Function),
        4 => Ok(SectionId::Table),
        5 => Ok(SectionId::Memory),
        6 => Ok(SectionId::Global),
        7 => Ok(SectionId::Export),
        8 => Ok(SectionId::Start),
        9 => Ok(SectionId::Element),
        10 => Ok(SectionId::Code),
        11 => Ok(SectionId::Data),
        12 => Ok(SectionId::DataCount),
        u => Err(WasmDecodeError::UnknownSectionId(u)),
    }?;
    Ok((sid, bytes))
}

fn decode_name(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmName>> {
    let (len, bytes) = decode_leb128(bytes)?;
    let (vec, bytes) = take_bytes_dyn(bytes, len as usize)?;
    let str = String::from_utf8(vec).map_err(WasmDecodeError::NonUtfName)?;
    let name = WasmName(str.into_boxed_str());
    Ok((name, bytes))
}

fn decode_custom_section(bytes: &[u8]) -> WasmDecodeResult<WasmCustom> {
    let (name, bytes) = decode_name(bytes)?;
    Ok(WasmCustom {
        name,
        bytes: bytes.into(),
    })
}

fn decode_value_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmValueType>> {
    let (tag, bytes) = take_byte(bytes)?;
    let vtype = match tag {
        0x7F => Ok(WasmValueType::Num(WasmNumType::I32)),
        0x7E => Ok(WasmValueType::Num(WasmNumType::I64)),
        0x7D => Ok(WasmValueType::Num(WasmNumType::F32)),
        0x7C => Ok(WasmValueType::Num(WasmNumType::F64)),
        0x7B => Ok(WasmValueType::Vec(WasmVecType::V128)),
        0x70 => Ok(WasmValueType::Ref(WasmRefType::FuncRef)),
        0x6F => Ok(WasmValueType::Ref(WasmRefType::ExternRef)),
        _ => Err(WasmDecodeError::InvalidValueType(tag)),
    }?;
    Ok((vtype, bytes))
}

fn decode_result_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmResultType>> {
    let mut res = WasmResultTypeBuilder::new();
    let (len, mut bytes) = decode_leb128(bytes)?;
    res.reserve(len as usize);
    for _ in 0..len {
        let (vtype, rest) = decode_value_type(bytes)?;
        res.push_value_type(vtype);
        bytes = rest;
    }
    Ok((res.build(), bytes))
}

fn decode_func_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmFuncType>> {
    let (marker, bytes) = take_byte(bytes)?;
    if marker != 0x60 {
        return Err(WasmDecodeError::InvalidFuncType(marker));
    }
    let (input_type, bytes) = decode_result_type(bytes)?;
    let (output_type, bytes) = decode_result_type(bytes)?;
    Ok((
        WasmFuncType {
            input_type,
            output_type,
        },
        bytes,
    ))
}

fn decode_type_section(bytes: &[u8], wmod: &mut WasmModuleBuilder) -> WasmDecodeResult<()> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    wmod.reserve_types(len as usize);
    for _ in 0..len {
        let (functype, rest) = decode_func_type(bytes)?;
        wmod.push_type(functype);
        bytes = rest;
    }
    Ok(())
}

fn decode_type_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmTypeIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmTypeIdx(idx), bytes))
}

fn decode_func_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmFuncIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmFuncIdx(idx), bytes))
}

fn decode_table_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmTableIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmTableIdx(idx), bytes))
}

fn decode_mem_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmMemIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmMemIdx(idx), bytes))
}

fn decode_global_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmGlobalIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmGlobalIdx(idx), bytes))
}

fn decode_ref_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmRefType>> {
    let (tag, bytes) = take_byte(bytes)?;
    match tag {
        0x70 => Ok((WasmRefType::FuncRef, bytes)),
        0x6F => Ok((WasmRefType::ExternRef, bytes)),
        _ => Err(WasmDecodeError::InvalidRefType(tag)),
    }
}

fn decode_limits(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmLimits>> {
    let (flag, bytes) = take_byte(bytes)?;
    match flag {
        0x00 => {
            let (min, bytes) = decode_leb128(bytes)?;
            Ok((WasmLimits { min, max: None }, bytes))
        }
        0x01 => {
            let (min, bytes) = decode_leb128(bytes)?;
            let (max, bytes) = decode_leb128(bytes)?;
            Ok((
                WasmLimits {
                    min,
                    max: Some(max),
                },
                bytes,
            ))
        }
        _ => Err(WasmDecodeError::InvalidLimits(flag)),
    }
}

fn decode_table_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmTableType>> {
    let (ref_type, bytes) = decode_ref_type(bytes)?;
    let (limits, bytes) = decode_limits(bytes)?;
    Ok((WasmTableType { ref_type, limits }, bytes))
}

fn decode_mem_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmMemType>> {
    let (limits, bytes) = decode_limits(bytes)?;
    Ok((WasmMemType { limits }, bytes))
}

fn decode_global_mutability(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmGlobalMutability>> {
    let (flag, bytes) = take_byte(bytes)?;
    match flag {
        0x00 => Ok((WasmGlobalMutability::Immutable, bytes)),
        0x01 => Ok((WasmGlobalMutability::Mutable, bytes)),
        _ => Err(WasmDecodeError::InvalidGlobalMutability(flag)),
    }
}

fn decode_global_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmGlobalType>> {
    let (val_type, bytes) = decode_value_type(bytes)?;
    let (mutability, bytes) = decode_global_mutability(bytes)?;
    Ok((
        WasmGlobalType {
            val_type,
            mutability,
        },
        bytes,
    ))
}

fn decode_import_desc(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmImportDesc>> {
    let (tag, bytes) = take_byte(bytes)?;
    match tag {
        0x00 => {
            let (type_idx, bytes) = decode_type_idx(bytes)?;
            Ok((WasmImportDesc::Func(type_idx), bytes))
        }
        0x01 => {
            let (table_type, bytes) = decode_table_type(bytes)?;
            Ok((WasmImportDesc::Table(table_type), bytes))
        }
        0x02 => {
            let (mem_type, bytes) = decode_mem_type(bytes)?;
            Ok((WasmImportDesc::Mem(mem_type), bytes))
        }
        0x03 => {
            let (global_type, bytes) = decode_global_type(bytes)?;
            Ok((WasmImportDesc::Global(global_type), bytes))
        }
        _ => Err(WasmDecodeError::InvalidImportDesc(tag)),
    }
}

fn decode_import(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmImport>> {
    let (module_name, bytes) = decode_name(bytes)?;
    let (item_name, bytes) = decode_name(bytes)?;
    let (desc, bytes) = decode_import_desc(bytes)?;
    Ok((
        WasmImport {
            module_name,
            item_name,
            desc,
        },
        bytes,
    ))
}

fn decode_import_section(bytes: &[u8], wmod: &mut WasmModuleBuilder) -> WasmDecodeResult<()> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    wmod.reserve_imports(len as usize);
    for _ in 0..len {
        let (import, rest) = decode_import(bytes)?;
        wmod.push_import(import);
        bytes = rest;
    }
    Ok(())
}

fn decode_func_section(bytes: &[u8], wmod: &mut WasmModuleBuilder) -> WasmDecodeResult<()> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    wmod.reserve_funcs(len as usize);
    for _ in 0..len {
        let (type_idx, rest) = decode_type_idx(bytes)?;
        wmod.push_func(type_idx);
        bytes = rest;
    }
    Ok(())
}

fn decode_code(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmCode>> {
    let (byte_size, bytes) = decode_leb128(bytes)?;
    let rest = &bytes[(byte_size as usize)..];
    // TODO: Decode code
    Ok((
        WasmCode {
            locals: vec![].into_boxed_slice(),
            body: WasmExpr(vec![].into_boxed_slice()),
        },
        rest,
    ))
}

fn decode_code_section(bytes: &[u8], wmod: &mut WasmModuleBuilder) -> WasmDecodeResult<()> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    wmod.reserve_code(len as usize);
    for _ in 0..len {
        let (code, rest) = decode_code(bytes)?;
        wmod.push_code(code);
        bytes = rest;
    }
    Ok(())
}

fn decode_table_section(bytes: &[u8], wmod: &mut WasmModuleBuilder) -> WasmDecodeResult<()> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    wmod.reserve_tables(len as usize);
    for _ in 0..len {
        let (table, rest) = decode_table_type(bytes)?;
        wmod.push_table(table);
        bytes = rest;
    }
    Ok(())
}

fn decode_memory_section(bytes: &[u8], wmod: &mut WasmModuleBuilder) -> WasmDecodeResult<()> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    wmod.reserve_mems(len as usize);
    for _ in 0..len {
        let (mem, rest) = decode_mem_type(bytes)?;
        wmod.push_mem(mem);
        bytes = rest;
    }
    Ok(())
}

fn decode_extended_instr(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmInstruction>> {
    let (opcode, bytes) = decode_leb128(bytes)?;
    use WasmInstruction::*;
    match opcode {
        0 => Ok((I32TruncSatF32S, bytes)),
        1 => Ok((I32TruncSatF32U, bytes)),
        2 => Ok((I32TruncSatF64S, bytes)),
        3 => Ok((I32TruncSatF64U, bytes)),
        4 => Ok((I64TruncSatF32S, bytes)),
        5 => Ok((I64TruncSatF32U, bytes)),
        6 => Ok((I64TruncSatF64S, bytes)),
        7 => Ok((I64TruncSatF64U, bytes)),
        8 => todo!("memory.init"),
        9 => todo!("data.drop"),
        10 => todo!("memory.copy"),
        11 => todo!("memory.fill"),
        12 => todo!("table.init"),
        13 => todo!("elem.drop"),
        14 => todo!("table.copy"),
        15 => todo!("table.grow"),
        16 => todo!("table.size"),
        17 => todo!("table.fill"),
        _ => Err(WasmDecodeError::UnknownExtendedOpcode(opcode)),
    }
}

fn decode_instr(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmInstruction>> {
    let (opcode, bytes) = take_byte(bytes)?;
    use WasmInstruction::*;
    match opcode {
        0x00 => Ok((Unreachable, bytes)),
        0x01 => Ok((Nop, bytes)),
        0x02 => todo!("block"),
        0x03 => todo!("loop"),
        0x04 => todo!("if"),
        0x0C => todo!("br l"),
        0x0D => todo!("br_if l"),
        0x0E => todo!("br_table l* lN"),
        0x0F => todo!("return"),
        0x10 => todo!("call f"),
        0x11 => todo!("call_indirect x y"),
        0xD0 => todo!("ref.null t"),
        0xD1 => todo!("ref.is_null"),
        0xD2 => todo!("ref.func x"),
        0x1A => todo!("drop"),
        0x1B => todo!("select"),
        0x1C => todo!("select t*"),
        0x20 => todo!("local.get x"),
        0x21 => todo!("local.set x"),
        0x22 => todo!("local.tee x"),
        0x23 => todo!("global.get x"),
        0x24 => todo!("global.set x"),
        0x25 => todo!("table.get x"),
        0x26 => todo!("table.set x"),
        0x28 => todo!("i32.load"),
        0x29 => todo!("i64.load"),
        0x2A => todo!("f32.load"),
        0x2B => todo!("f64.load"),
        0x2C => todo!("i32.load8_s"),
        0x2D => todo!("i32.load8_u"),
        0x2E => todo!("i32.load16_s"),
        0x2F => todo!("i32.load16_u"),
        0x30 => todo!("i64.load8_s"),
        0x31 => todo!("i64.load8_u"),
        0x32 => todo!("i64.load16_s"),
        0x33 => todo!("i64.load16_u"),
        0x34 => todo!("i64.load32_s"),
        0x35 => todo!("i64.load32_u"),
        0x36 => todo!("i32.store"),
        0x37 => todo!("i64.store"),
        0x38 => todo!("f32.store"),
        0x39 => todo!("f64.store"),
        0x3A => todo!("i32.store8"),
        0x3B => todo!("i32.store16"),
        0x3C => todo!("i64.store8"),
        0x3D => todo!("i64.store16"),
        0x3E => todo!("i64.store32"),
        0x3F => todo!("memory.size"),
        0x40 => todo!("memory.grow"),
        0x41 => todo!("i32.const"),
        0x42 => todo!("i64.const"),
        0x43 => todo!("f32.const"),
        0x44 => todo!("f64.const"),
        0x45 => Ok((I32EqZ, bytes)),
        0x46 => Ok((I32Eq, bytes)),
        0x47 => Ok((I32Neq, bytes)),
        0x48 => Ok((I32LtS, bytes)),
        0x49 => Ok((I32LtU, bytes)),
        0x4A => Ok((I32GtS, bytes)),
        0x4B => Ok((I32GtU, bytes)),
        0x4C => Ok((I32LeS, bytes)),
        0x4D => Ok((I32LeU, bytes)),
        0x4E => Ok((I32GeS, bytes)),
        0x4F => Ok((I32GeU, bytes)),
        0x50 => Ok((I64EqZ, bytes)),
        0x51 => Ok((I64Eq, bytes)),
        0x52 => Ok((I64Neq, bytes)),
        0x53 => Ok((I64LtS, bytes)),
        0x54 => Ok((I64LtU, bytes)),
        0x55 => Ok((I64GtS, bytes)),
        0x56 => Ok((I64GtU, bytes)),
        0x57 => Ok((I64LeS, bytes)),
        0x58 => Ok((I64LeU, bytes)),
        0x59 => Ok((I64GeS, bytes)),
        0x5A => Ok((I64GeU, bytes)),
        0x5B => Ok((F32Eq, bytes)),
        0x5C => Ok((F32Neq, bytes)),
        0x5D => Ok((F32Lt, bytes)),
        0x5E => Ok((F32Gt, bytes)),
        0x5F => Ok((F32Le, bytes)),
        0x60 => Ok((F32Ge, bytes)),
        0x61 => Ok((F64Eq, bytes)),
        0x62 => Ok((F64Neq, bytes)),
        0x64 => Ok((F64Gt, bytes)),
        0x65 => Ok((F64Le, bytes)),
        0x66 => Ok((F64Ge, bytes)),
        0x67 => Ok((I32Clz, bytes)),
        0x68 => Ok((I32Ctz, bytes)),
        0x69 => Ok((I32Popcnt, bytes)),
        0x6A => Ok((I32Add, bytes)),
        0x6B => Ok((I32Sub, bytes)),
        0x6C => Ok((I32Mul, bytes)),
        0x6D => Ok((I32DivS, bytes)),
        0x6E => Ok((I32DivU, bytes)),
        0x6F => Ok((I32RemS, bytes)),
        0x70 => Ok((I32RemU, bytes)),
        0x71 => Ok((I32And, bytes)),
        0x72 => Ok((I32Or, bytes)),
        0x73 => Ok((I32Xor, bytes)),
        0x74 => Ok((I32Shl, bytes)),
        0x75 => Ok((I32ShrS, bytes)),
        0x76 => Ok((I32ShrU, bytes)),
        0x77 => Ok((I32Rotl, bytes)),
        0x78 => Ok((I32Rotr, bytes)),
        0x79 => Ok((I64Clz, bytes)),
        0x7A => Ok((I64Ctz, bytes)),
        0x7B => Ok((I64Popcnt, bytes)),
        0x7C => Ok((I64Add, bytes)),
        0x7D => Ok((I64Sub, bytes)),
        0x7E => Ok((I64Mul, bytes)),
        0x7F => Ok((I64DivS, bytes)),
        0x80 => Ok((I64DivU, bytes)),
        0x81 => Ok((I64RemS, bytes)),
        0x82 => Ok((I64RemU, bytes)),
        0x83 => Ok((I64And, bytes)),
        0x84 => Ok((I64Or, bytes)),
        0x85 => Ok((I64Xor, bytes)),
        0x86 => Ok((I64Shl, bytes)),
        0x87 => Ok((I64ShrS, bytes)),
        0x88 => Ok((I64ShrU, bytes)),
        0x89 => Ok((I64Rotl, bytes)),
        0x8A => Ok((I64Rotr, bytes)),
        0x8B => Ok((F32Abs, bytes)),
        0x8C => Ok((F32Neg, bytes)),
        0x8D => Ok((F32Ceil, bytes)),
        0x8E => Ok((F32Floor, bytes)),
        0x8F => Ok((F32Trunc, bytes)),
        0x90 => Ok((F32Nearest, bytes)),
        0x91 => Ok((F32Sqrt, bytes)),
        0x92 => Ok((F32Add, bytes)),
        0x93 => Ok((F32Sub, bytes)),
        0x94 => Ok((F32Mul, bytes)),
        0x95 => Ok((F32Div, bytes)),
        0x96 => Ok((F32Min, bytes)),
        0x97 => Ok((F32Max, bytes)),
        0x98 => Ok((F32CopySign, bytes)),
        0x99 => Ok((F64Abs, bytes)),
        0x9A => Ok((F64Neg, bytes)),
        0x9B => Ok((F64Ceil, bytes)),
        0x9C => Ok((F64Floor, bytes)),
        0x9D => Ok((F64Trunc, bytes)),
        0x9E => Ok((F64Nearest, bytes)),
        0x9F => Ok((F64Sqrt, bytes)),
        0xA0 => Ok((F64Add, bytes)),
        0xA1 => Ok((F64Sub, bytes)),
        0xA2 => Ok((F64Mul, bytes)),
        0xA3 => Ok((F64Div, bytes)),
        0xA4 => Ok((F64Min, bytes)),
        0xA5 => Ok((F64Max, bytes)),
        0xA6 => Ok((F64CopySign, bytes)),
        0xA7 => Ok((I32WrapI64, bytes)),
        0xA8 => Ok((I32TruncF32S, bytes)),
        0xA9 => Ok((I32TruncF32U, bytes)),
        0xAA => Ok((I32TruncF64S, bytes)),
        0xAB => Ok((I32TruncF64U, bytes)),
        0xAC => Ok((I64ExtendI32S, bytes)),
        0xAD => Ok((I64ExtendI32U, bytes)),
        0xAE => Ok((I64TruncF32S, bytes)),
        0xAF => Ok((I64TruncF32U, bytes)),
        0xB0 => Ok((I64TruncF64S, bytes)),
        0xB1 => Ok((I64TruncF64U, bytes)),
        0xB2 => Ok((F32ConvertI32S, bytes)),
        0xB3 => Ok((F32ConvertI32U, bytes)),
        0xB4 => Ok((F32ConvertI64S, bytes)),
        0xB5 => Ok((F32ConvertI64U, bytes)),
        0xB6 => Ok((F32DemoteF64, bytes)),
        0xB7 => Ok((F64ConvertI32S, bytes)),
        0xB8 => Ok((F64ConvertI32U, bytes)),
        0xB9 => Ok((F64ConvertI64S, bytes)),
        0xBA => Ok((F64ConvertI64U, bytes)),
        0xBB => Ok((F64PromoteF32, bytes)),
        0xBC => Ok((I32ReinterpretF32, bytes)),
        0xBD => Ok((I64ReinterpretF64, bytes)),
        0xBE => Ok((F32ReinterpretI32, bytes)),
        0xBF => Ok((F64ReinterpretI64, bytes)),
        0xC0 => Ok((I32Extend8S, bytes)),
        0xC1 => Ok((I32Extend16S, bytes)),
        0xC2 => Ok((I64Extend8S, bytes)),
        0xC3 => Ok((I64Extend16S, bytes)),
        0xC4 => Ok((I64Extend32S, bytes)),
        0xFC => decode_extended_instr(bytes),
        _ => Err(WasmDecodeError::UnknownOpcode(opcode)),
    }
}

fn decode_expr(mut bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmExpr>> {
    let mut expr = WasmExprBuilder::new();
    loop {
        let (instr, rest) = decode_instr(bytes)?;
        bytes = rest;
        match instr {
            WasmInstruction::ExprEnd => break,
            _ => {}
        }
        expr.push_instr(instr);
    }
    Ok((expr.build(), bytes))
}

fn decode_global(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmGlobal>> {
    let (global_type, bytes) = decode_global_type(bytes)?;
    let (expr, bytes) = decode_expr(bytes)?;
    Ok((
        WasmGlobal {
            global_type,
            init: expr,
        },
        bytes,
    ))
}

fn decode_global_section(bytes: &[u8], wmod: &mut WasmModuleBuilder) -> WasmDecodeResult<()> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    wmod.reserve_globals(len as usize);
    for _ in 0..len {
        let (global, rest) = decode_global(bytes)?;
        wmod.push_global(global);
        bytes = rest;
    }
    Ok(())
}

fn decode_export_desc(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmExportDesc>> {
    let (tag, bytes) = take_byte(bytes)?;
    match tag {
        0x00 => {
            let (func_idx, bytes) = decode_func_idx(bytes)?;
            Ok((WasmExportDesc::Func(func_idx), bytes))
        }
        0x01 => {
            let (table_idx, bytes) = decode_table_idx(bytes)?;
            Ok((WasmExportDesc::Table(table_idx), bytes))
        }
        0x02 => {
            let (mem_idx, bytes) = decode_mem_idx(bytes)?;
            Ok((WasmExportDesc::Mem(mem_idx), bytes))
        }
        0x03 => {
            let (global_idx, bytes) = decode_global_idx(bytes)?;
            Ok((WasmExportDesc::Global(global_idx), bytes))
        }
        _ => Err(WasmDecodeError::InvalidExportDesc(tag)),
    }
}

fn decode_export(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmExport>> {
    let (name, bytes) = decode_name(bytes)?;
    let (desc, bytes) = decode_export_desc(bytes)?;
    Ok((WasmExport { name, desc }, bytes))
}

fn decode_export_section(bytes: &[u8], wmod: &mut WasmModuleBuilder) -> WasmDecodeResult<()> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    wmod.reserve_exports(len as usize);
    for _ in 0..len {
        let (export, rest) = decode_export(bytes)?;
        wmod.push_export(export);
        bytes = rest;
    }
    Ok(())
}

fn decode_elem(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmElem>> {
    todo!()
}

fn decode_element_section(bytes: &[u8], wmod: &mut WasmModuleBuilder) -> WasmDecodeResult<()> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    wmod.reserve_elems(len as usize);
    for _ in 0..len {
        let (elem, rest) = decode_elem(bytes)?;
        wmod.push_elem(elem);
        bytes = rest;
    }
    Ok(())
}

fn decode_data(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmData>> {
    todo!()
}

fn decode_data_section(bytes: &[u8], wmod: &mut WasmModuleBuilder) -> WasmDecodeResult<()> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    wmod.reserve_datas(len as usize);
    for _ in 0..len {
        let (data, rest) = decode_data(bytes)?;
        wmod.push_data(data);
        bytes = rest;
    }
    Ok(())
}

fn decode_section<'b>(
    bytes: &'b [u8],
    wmod: &mut WasmModuleBuilder,
) -> WasmDecodeResult<Decoded<'b, ()>> {
    let (sid, bytes) = decode_section_id(bytes)?;
    let (len, bytes) = decode_leb128(bytes)?;
    let section = &bytes[0..(len as usize)];
    let rest = &bytes[(len as usize)..];
    match sid {
        SectionId::Custom => {
            let custom = decode_custom_section(section)?;
            wmod.push_custom(custom);
            Ok(((), rest))
        }
        SectionId::Type => {
            decode_type_section(section, wmod)?;
            Ok(((), rest))
        }
        SectionId::Import => {
            decode_import_section(section, wmod)?;
            Ok(((), rest))
        }
        SectionId::Function => {
            decode_func_section(section, wmod)?;
            Ok(((), rest))
        }
        SectionId::Table => {
            decode_table_section(section, wmod)?;
            Ok(((), rest))
        }
        SectionId::Memory => {
            decode_memory_section(section, wmod)?;
            Ok(((), rest))
        }
        SectionId::Global => {
            //decode_global_section(section, wmod)?;
            Ok(((), rest))
        }
        SectionId::Export => {
            decode_export_section(section, wmod)?;
            Ok(((), rest))
        }
        SectionId::Start => {
            let (func_idx, _) = decode_func_idx(section)?;
            wmod.start(func_idx);
            Ok(((), rest))
        }
        SectionId::Element => {
            //decode_element_section(section, wmod)?;
            Ok(((), rest))
        }
        SectionId::Data => {
            //decode_data_section(section, wmod)?;
            Ok(((), rest))
        }
        SectionId::DataCount => {
            let (datacount, _) = decode_leb128(section)?;
            wmod.datacount(datacount);
            Ok(((), rest))
        }
        SectionId::Code => {
            decode_code_section(section, wmod)?;
            Ok(((), rest))
        }
    }
}

fn decode_sections<'b>(
    mut bytes: &'b [u8],
    wmod: &mut WasmModuleBuilder,
) -> WasmDecodeResult<Decoded<'b, ()>> {
    while !bytes.is_empty() {
        let (_, rest) = decode_section(bytes, wmod)?;
        bytes = rest;
    }
    Ok(((), bytes))
}

fn decode_magic_bytes(bytes: &[u8]) -> WasmDecodeResult<Decoded<()>> {
    let (buf, bytes) = take_bytes::<4>(bytes)?;
    match buf {
        [0, b'a', b's', b'm'] => Ok(((), bytes)),
        _ => Err(WasmDecodeError::MagicBytes),
    }
}

fn decode_version(bytes: &[u8]) -> WasmDecodeResult<Decoded<u32>> {
    let (buf, bytes) = take_bytes::<4>(bytes)?;
    Ok((u32::from_le_bytes(buf), bytes))
}

pub fn decode(bytes: &[u8]) -> WasmDecodeResult<WasmModule> {
    let mut wmod = WasmModuleBuilder::new();
    let (_, bytes) = decode_magic_bytes(bytes)?;
    let (version, bytes) = decode_version(bytes)?;
    if version != 1 {
        return Err(WasmDecodeError::UnsupportedVersion(version));
    }
    wmod.version(version);
    decode_sections(bytes, &mut wmod)?;
    Ok(wmod.build())
}
