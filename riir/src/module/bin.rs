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
    InvalidBlockType,
    InvalidConst,
    InvalidData,
    InvalidElem,
    UnknownOpcode(u8),
    UnknownExtendedOpcode(u32),
    UnsupportedMemIdx(u32),
    UnexpectedByte { expected: u8, actual: u8 },
    UnexpectedEof,
}

pub type WasmDecodeResult<T> = Result<T, WasmDecodeError>;

type Decoded<'b, T> = (T, &'b [u8]);

fn take_bytes<const N: usize>(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, [u8; N]>> {
    if bytes.len() < N {
        return Err(WasmDecodeError::UnexpectedEof);
    }
    let mut buf = [0u8; N];
    buf.copy_from_slice(&bytes[0..N]);
    Ok((buf, &bytes[N..]))
}

fn take_byte(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, u8>> {
    let (buf, bytes) = take_bytes::<1>(bytes)?;
    Ok((buf[0], bytes))
}

fn take_byte_exact<const B: u8>(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, ()>> {
    let (byte, bytes) = take_byte(bytes)?;
    if byte == B {
        Ok(((), bytes))
    } else {
        Err(WasmDecodeError::UnexpectedByte {
            expected: B,
            actual: byte,
        })
    }
}

fn take_bytes_dyn(bytes: &[u8], n: usize) -> WasmDecodeResult<Decoded<'_, Vec<u8>>> {
    if bytes.len() < n {
        return Err(WasmDecodeError::UnexpectedEof);
    }
    let mut buf = Vec::with_capacity(n);
    buf.extend_from_slice(&bytes[0..n]);
    Ok((buf, &bytes[n..]))
}

fn decode_leb128(mut bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, u32>> {
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

fn decode_leb128_signed(mut bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, i64>> {
    let mut result = 0;
    let mut shift = 0;

    let byte = loop {
        let (byte, rest) = take_byte(bytes)?;
        bytes = rest;
        result |= ((byte & !(1 << 7)) as i64) << shift;
        shift += 7;
        if (byte & (1 << 7)) == 0 {
            break byte;
        }
    };

    if (shift < 64) && ((byte & 0x40) != 0) {
        result |= (u64::MAX as i64) << shift;
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

fn decode_section_id(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, SectionId>> {
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

fn decode_name(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmName>> {
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

fn decode_value_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmValueType>> {
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

fn decode_result_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmResultType>> {
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

fn decode_func_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmFuncType>> {
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

fn decode_type_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmTypeIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmTypeIdx(idx), bytes))
}

fn decode_func_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmFuncIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmFuncIdx(idx), bytes))
}

fn decode_table_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmTableIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmTableIdx(idx), bytes))
}

fn decode_elem_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmElemIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmElemIdx(idx), bytes))
}

fn decode_mem_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmMemIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmMemIdx(idx), bytes))
}

fn decode_mem_idx_zero(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, ()>> {
    let (mem_idx, bytes) = decode_mem_idx(bytes)?;
    if mem_idx.0 == 0 {
        Ok(((), bytes))
    } else {
        Err(WasmDecodeError::UnsupportedMemIdx(mem_idx.0))
    }
}

fn decode_data_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmDataIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmDataIdx(idx), bytes))
}

fn decode_global_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmGlobalIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmGlobalIdx(idx), bytes))
}

fn decode_label_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmLabelIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmLabelIdx(idx), bytes))
}

fn decode_local_idx(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmLocalIdx>> {
    let (idx, bytes) = decode_leb128(bytes)?;
    Ok((WasmLocalIdx(idx), bytes))
}

fn decode_ref_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmRefType>> {
    let (tag, bytes) = take_byte(bytes)?;
    match tag {
        0x70 => Ok((WasmRefType::FuncRef, bytes)),
        0x6F => Ok((WasmRefType::ExternRef, bytes)),
        _ => Err(WasmDecodeError::InvalidRefType(tag)),
    }
}

fn decode_limits(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmLimits>> {
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

fn decode_table_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmTableType>> {
    let (ref_type, bytes) = decode_ref_type(bytes)?;
    let (limits, bytes) = decode_limits(bytes)?;
    Ok((WasmTableType { ref_type, limits }, bytes))
}

fn decode_mem_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmMemType>> {
    let (limits, bytes) = decode_limits(bytes)?;
    Ok((WasmMemType { limits }, bytes))
}

fn decode_global_mutability(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmGlobalMutability>> {
    let (flag, bytes) = take_byte(bytes)?;
    match flag {
        0x00 => Ok((WasmGlobalMutability::Immutable, bytes)),
        0x01 => Ok((WasmGlobalMutability::Mutable, bytes)),
        _ => Err(WasmDecodeError::InvalidGlobalMutability(flag)),
    }
}

fn decode_global_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmGlobalType>> {
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

fn decode_import_desc(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmImportDesc>> {
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

fn decode_import(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmImport>> {
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

fn decode_locals(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, Box<[WasmValueType]>>> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    let mut locals = Vec::new();
    for _ in 0..len {
        let (n, rest) = decode_leb128(bytes)?;
        let (t, rest) = decode_value_type(rest)?;
        bytes = rest;
        for _ in 0..n {
            locals.push(t);
        }
    }
    Ok((locals.into_boxed_slice(), bytes))
}

fn decode_code(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmCode>> {
    let (code_size, bytes) = decode_leb128(bytes)?;
    let (bytes, rest) = bytes.split_at(code_size as usize);
    let (locals, bytes) = decode_locals(bytes)?;
    let body = decode_expr(bytes)?;
    Ok((WasmCode { locals, body }, rest))
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

fn decode_block_type(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmBlockType>> {
    if let Ok((_, bytes)) = take_byte_exact::<0x40>(bytes) {
        Ok((WasmBlockType::InlineType(None), bytes))
    } else if let Ok((val_type, bytes)) = decode_value_type(bytes) {
        Ok((WasmBlockType::InlineType(Some(val_type)), bytes))
    } else {
        let (s33, bytes) = decode_leb128_signed(bytes)?;
        if s33 > 0 && s33 < (u32::MAX as i64) {
            Ok((WasmBlockType::TypeRef(WasmTypeIdx(s33 as u32)), bytes))
        } else {
            Err(WasmDecodeError::InvalidBlockType)
        }
    }
}

fn decode_memarg(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmMemArg>> {
    let (align, bytes) = decode_leb128(bytes)?;
    let (offset, bytes) = decode_leb128(bytes)?;
    Ok((WasmMemArg { align, offset }, bytes))
}

fn decode_label_indices(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, Box<[WasmLabelIdx]>>> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    let mut indices = Vec::with_capacity(len as usize);
    for _ in 0..len {
        let (label_idx, rest) = decode_label_idx(bytes)?;
        bytes = rest;
        indices.push(label_idx);
    }
    Ok((indices.into_boxed_slice(), bytes))
}

fn decode_extended_instr(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmInstructionRaw>> {
    let (opcode, bytes) = decode_leb128(bytes)?;
    use WasmInstructionRepr::*;
    match opcode {
        0 => Ok((I32TruncSatF32S, bytes)),
        1 => Ok((I32TruncSatF32U, bytes)),
        2 => Ok((I32TruncSatF64S, bytes)),
        3 => Ok((I32TruncSatF64U, bytes)),
        4 => Ok((I64TruncSatF32S, bytes)),
        5 => Ok((I64TruncSatF32U, bytes)),
        6 => Ok((I64TruncSatF64S, bytes)),
        7 => Ok((I64TruncSatF64U, bytes)),
        8 => {
            let (data_idx, bytes) = decode_data_idx(bytes)?;
            let (_, bytes) = decode_mem_idx_zero(bytes)?;
            Ok((MemoryInit { data_idx }, bytes))
        }
        9 => {
            let (data_idx, bytes) = decode_data_idx(bytes)?;
            Ok((DataDrop { data_idx }, bytes))
        }
        10 => {
            let (_, bytes) = decode_mem_idx_zero(bytes)?;
            let (_, bytes) = decode_mem_idx_zero(bytes)?;
            Ok((MemoryCopy, bytes))
        }
        11 => {
            let (_, bytes) = decode_mem_idx_zero(bytes)?;
            Ok((MemoryFill, bytes))
        }
        12 => {
            let (elem_idx, bytes) = decode_elem_idx(bytes)?;
            let (table_idx, bytes) = decode_table_idx(bytes)?;
            Ok((
                TableInit {
                    elem_idx,
                    table_idx,
                },
                bytes,
            ))
        }
        13 => {
            let (elem_idx, bytes) = decode_elem_idx(bytes)?;
            Ok((ElemDrop { elem_idx }, bytes))
        }
        14 => {
            let (dst, bytes) = decode_table_idx(bytes)?;
            let (src, bytes) = decode_table_idx(bytes)?;
            Ok((TableCopy { dst, src }, bytes))
        }
        15 => {
            let (table_idx, bytes) = decode_table_idx(bytes)?;
            Ok((TableGrow { table_idx }, bytes))
        }
        16 => {
            let (table_idx, bytes) = decode_table_idx(bytes)?;
            Ok((TableSize { table_idx }, bytes))
        }
        17 => {
            let (table_idx, bytes) = decode_table_idx(bytes)?;
            Ok((TableFill { table_idx }, bytes))
        }
        _ => Err(WasmDecodeError::UnknownExtendedOpcode(opcode)),
    }
}

fn decode_instr(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmInstructionRaw>> {
    let (opcode, bytes) = take_byte(bytes)?;
    use WasmInstructionRepr::*;
    match opcode {
        0x00 => Ok((Unreachable, bytes)),
        0x01 => Ok((Nop, bytes)),
        0x02 => {
            let (block_type, bytes) = decode_block_type(bytes)?;
            Ok((
                Block {
                    block_type,
                    imm: (),
                },
                bytes,
            ))
        }
        0x03 => {
            let (block_type, bytes) = decode_block_type(bytes)?;
            Ok((
                Loop {
                    block_type,
                    imm: (),
                },
                bytes,
            ))
        }
        0x04 => {
            let (block_type, bytes) = decode_block_type(bytes)?;
            Ok((
                If {
                    block_type,
                    imm: (),
                },
                bytes,
            ))
        }
        0x05 => Ok((Else, bytes)),
        0x0B => Ok((ExprEnd, bytes)),
        0x0C => {
            let (label_idx, bytes) = decode_label_idx(bytes)?;
            Ok((Break { label_idx }, bytes))
        }
        0x0D => {
            let (label_idx, bytes) = decode_label_idx(bytes)?;
            Ok((BreakIf { label_idx }, bytes))
        }
        0x0E => {
            let (labels, bytes) = decode_label_indices(bytes)?;
            let (default_label, bytes) = decode_label_idx(bytes)?;
            Ok((
                BreakTable {
                    labels,
                    default_label,
                },
                bytes,
            ))
        }
        0x0F => Ok((Return, bytes)),
        0x10 => {
            let (func_idx, bytes) = decode_func_idx(bytes)?;
            Ok((Call { func_idx }, bytes))
        }
        0x11 => {
            let (type_idx, bytes) = decode_type_idx(bytes)?;
            let (table_idx, bytes) = decode_table_idx(bytes)?;
            Ok((
                CallIndirect {
                    type_idx,
                    table_idx,
                },
                bytes,
            ))
        }
        0xD0 => {
            let (ref_type, bytes) = decode_ref_type(bytes)?;
            Ok((RefNull { ref_type }, bytes))
        }
        0xD1 => Ok((RefIsNull, bytes)),
        0xD2 => {
            let (func_idx, bytes) = decode_func_idx(bytes)?;
            Ok((RefFunc { func_idx }, bytes))
        }
        0x1A => Ok((Drop, bytes)),
        0x1B => Ok((
            Select {
                value_types: Box::new([]),
            },
            bytes,
        )),
        0x1C => {
            let (res, bytes) = decode_result_type(bytes)?;
            Ok((Select { value_types: res.0 }, bytes))
        }
        0x20 => {
            let (local_idx, bytes) = decode_local_idx(bytes)?;
            Ok((LocalGet { local_idx }, bytes))
        }
        0x21 => {
            let (local_idx, bytes) = decode_local_idx(bytes)?;
            Ok((LocalSet { local_idx }, bytes))
        }
        0x22 => {
            let (local_idx, bytes) = decode_local_idx(bytes)?;
            Ok((LocalTee { local_idx }, bytes))
        }
        0x23 => {
            let (global_idx, bytes) = decode_global_idx(bytes)?;
            Ok((GlobalGet { global_idx }, bytes))
        }
        0x24 => {
            let (global_idx, bytes) = decode_global_idx(bytes)?;
            Ok((GlobalSet { global_idx }, bytes))
        }
        0x25 => {
            let (table_idx, bytes) = decode_table_idx(bytes)?;
            Ok((TableGet { table_idx }, bytes))
        }
        0x26 => {
            let (table_idx, bytes) = decode_table_idx(bytes)?;
            Ok((TableSet { table_idx }, bytes))
        }
        0x28 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I32Load { memarg }, bytes))
        }
        0x29 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I64Load { memarg }, bytes))
        }
        0x2A => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((F32Load { memarg }, bytes))
        }
        0x2B => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((F64Load { memarg }, bytes))
        }
        0x2C => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I32Load8S { memarg }, bytes))
        }
        0x2D => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I32Load8U { memarg }, bytes))
        }
        0x2E => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I32Load16S { memarg }, bytes))
        }
        0x2F => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I32Load16U { memarg }, bytes))
        }
        0x30 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I64Load8S { memarg }, bytes))
        }
        0x31 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I64Load8U { memarg }, bytes))
        }
        0x32 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I64Load16S { memarg }, bytes))
        }
        0x33 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I64Load16U { memarg }, bytes))
        }
        0x34 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I64Load32S { memarg }, bytes))
        }
        0x35 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I64Load32U { memarg }, bytes))
        }
        0x36 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I32Store { memarg }, bytes))
        }
        0x37 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I64Store { memarg }, bytes))
        }
        0x38 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((F32Store { memarg }, bytes))
        }
        0x39 => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((F64Store { memarg }, bytes))
        }
        0x3A => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I32Store8 { memarg }, bytes))
        }
        0x3B => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I32Store16 { memarg }, bytes))
        }
        0x3C => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I64Store8 { memarg }, bytes))
        }
        0x3D => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I64Store16 { memarg }, bytes))
        }
        0x3E => {
            let (memarg, bytes) = decode_memarg(bytes)?;
            Ok((I64Store32 { memarg }, bytes))
        }
        0x3F => {
            let (_, bytes) = decode_mem_idx_zero(bytes)?;
            Ok((MemorySize, bytes))
        }
        0x40 => {
            let (_, bytes) = decode_mem_idx_zero(bytes)?;
            Ok((MemoryGrow, bytes))
        }
        0x41 => {
            let (v, bytes) = decode_leb128_signed(bytes)?;
            if v >= (i32::MIN as i64) && v <= (i32::MAX as i64) {
                Ok((I32Const { val: v as i32 }, bytes))
            } else {
                Err(WasmDecodeError::InvalidConst)
            }
        }
        0x42 => {
            let (val, bytes) = decode_leb128_signed(bytes)?;
            Ok((I64Const { val }, bytes))
        }
        0x43 => {
            let (buf, bytes) = take_bytes::<4>(bytes)?;
            let val = f32::from_le_bytes(buf);
            Ok((F32Const { val }, bytes))
        }
        0x44 => {
            let (buf, bytes) = take_bytes::<8>(bytes)?;
            let val = f64::from_le_bytes(buf);
            Ok((F64Const { val }, bytes))
        }
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

fn decode_expr(mut bytes: &[u8]) -> WasmDecodeResult<Box<[WasmInstructionRaw]>> {
    let mut expr = WasmExprBuilder::new();
    while !bytes.is_empty() {
        let (instr, rest) = decode_instr(bytes)?;
        expr.push_instr(instr);
        bytes = rest;
    }
    Ok(expr.build())
}

fn decode_const_expr(mut bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, Box<[WasmInstructionRaw]>>> {
    let mut expr = WasmExprBuilder::new();
    loop {
        let (instr, rest) = decode_instr(bytes)?;
        bytes = rest;
        if let WasmInstructionRepr::ExprEnd = expr.push_instr(instr) {
            break;
        }
    }
    Ok((expr.build(), bytes))
}

fn decode_global(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmGlobal<WasmInstructionRaw>>> {
    let (global_type, bytes) = decode_global_type(bytes)?;
    let (expr, bytes) = decode_const_expr(bytes)?;
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

fn decode_export_desc(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmExportDesc>> {
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

fn decode_export(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmExport>> {
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

fn decode_elem_init_func_refs(
    bytes: &[u8],
) -> WasmDecodeResult<Decoded<'_, Box<[Box<WasmExprRaw>]>>> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    let mut exprs = Vec::with_capacity(len as usize);
    for _ in 0..len {
        let (func_idx, rest) = decode_func_idx(bytes)?;
        bytes = rest;
        let mut expr = WasmExprBuilder::new();
        expr.push_instr(WasmInstructionRepr::RefFunc { func_idx });
        expr.push_instr(WasmInstructionRepr::ExprEnd);
        exprs.push(expr.build());
    }
    Ok((exprs.into_boxed_slice(), bytes))
}

fn decode_elem_init_exprs(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, Box<[Box<WasmExprRaw>]>>> {
    let (len, mut bytes) = decode_leb128(bytes)?;
    let mut exprs = Vec::with_capacity(len as usize);
    for _ in 0..len {
        let (expr, rest) = decode_const_expr(bytes)?;
        bytes = rest;
        exprs.push(expr);
    }
    Ok((exprs.into_boxed_slice(), bytes))
}

fn decode_elem_kind(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmRefType>> {
    let (_, bytes) = take_byte_exact::<0x00>(bytes)?;
    Ok((WasmRefType::FuncRef, bytes))
}

fn decode_elem(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmElem<WasmInstructionRaw>>> {
    let (tag, bytes) = decode_leb128(bytes)?;
    match tag {
        0 => {
            let (offset_expr, bytes) = decode_const_expr(bytes)?;
            let (init, bytes) = decode_elem_init_func_refs(bytes)?;
            Ok((
                WasmElem {
                    ref_type: WasmRefType::FuncRef,
                    init,
                    elem_mode: WasmElemMode::Active {
                        table_idx: WasmTableIdx(0),
                        offset_expr,
                    },
                },
                bytes,
            ))
        }
        1 => {
            let (ref_type, bytes) = decode_elem_kind(bytes)?;
            let (init, bytes) = decode_elem_init_func_refs(bytes)?;
            Ok((
                WasmElem {
                    ref_type,
                    init,
                    elem_mode: WasmElemMode::Passive,
                },
                bytes,
            ))
        }
        2 => {
            let (table_idx, bytes) = decode_table_idx(bytes)?;
            let (offset_expr, bytes) = decode_const_expr(bytes)?;
            let (ref_type, bytes) = decode_elem_kind(bytes)?;
            let (init, bytes) = decode_elem_init_func_refs(bytes)?;
            Ok((
                WasmElem {
                    ref_type,
                    init,
                    elem_mode: WasmElemMode::Active {
                        table_idx,
                        offset_expr,
                    },
                },
                bytes,
            ))
        }
        3 => {
            let (ref_type, bytes) = decode_elem_kind(bytes)?;
            let (init, bytes) = decode_elem_init_func_refs(bytes)?;
            Ok((
                WasmElem {
                    ref_type,
                    init,
                    elem_mode: WasmElemMode::Passive,
                },
                bytes,
            ))
        }
        4 => {
            let (offset_expr, bytes) = decode_const_expr(bytes)?;
            let (init, bytes) = decode_elem_init_exprs(bytes)?;
            Ok((
                WasmElem {
                    ref_type: WasmRefType::FuncRef,
                    init,
                    elem_mode: WasmElemMode::Active {
                        table_idx: WasmTableIdx(0),
                        offset_expr,
                    },
                },
                bytes,
            ))
        }
        5 => {
            let (ref_type, bytes) = decode_ref_type(bytes)?;
            let (init, bytes) = decode_elem_init_exprs(bytes)?;
            Ok((
                WasmElem {
                    ref_type,
                    init,
                    elem_mode: WasmElemMode::Passive,
                },
                bytes,
            ))
        }
        6 => {
            let (table_idx, bytes) = decode_table_idx(bytes)?;
            let (offset_expr, bytes) = decode_const_expr(bytes)?;
            let (ref_type, bytes) = decode_ref_type(bytes)?;
            let (init, bytes) = decode_elem_init_exprs(bytes)?;
            Ok((
                WasmElem {
                    ref_type,
                    init,
                    elem_mode: WasmElemMode::Active {
                        table_idx,
                        offset_expr,
                    },
                },
                bytes,
            ))
        }
        7 => {
            let (ref_type, bytes) = decode_ref_type(bytes)?;
            let (init, bytes) = decode_elem_init_exprs(bytes)?;
            Ok((
                WasmElem {
                    ref_type,
                    init,
                    elem_mode: WasmElemMode::Declarative,
                },
                bytes,
            ))
        }
        _ => Err(WasmDecodeError::InvalidElem),
    }
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

fn decode_data_bytes(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, Box<[u8]>>> {
    let (len, bytes) = decode_leb128(bytes)?;
    let (data_bytes, bytes) = take_bytes_dyn(bytes, len as usize)?;
    Ok((data_bytes.into_boxed_slice(), bytes))
}

fn decode_data(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, WasmData<WasmInstructionRaw>>> {
    let (tag, bytes) = decode_leb128(bytes)?;
    match tag {
        0 => {
            let (offset_expr, bytes) = decode_const_expr(bytes)?;
            let (data_bytes, bytes) = decode_data_bytes(bytes)?;
            let mode = WasmDataMode::Active {
                mem_idx: WasmMemIdx(0),
                offset_expr,
            };
            Ok((
                WasmData {
                    mode,
                    bytes: data_bytes,
                },
                bytes,
            ))
        }
        1 => {
            let (data_bytes, bytes) = decode_data_bytes(bytes)?;
            Ok((
                WasmData {
                    mode: WasmDataMode::Passive,
                    bytes: data_bytes,
                },
                bytes,
            ))
        }
        2 => {
            let (mem_idx, bytes) = decode_mem_idx(bytes)?;
            let (offset_expr, bytes) = decode_const_expr(bytes)?;
            let (data_bytes, bytes) = decode_data_bytes(bytes)?;
            let mode = WasmDataMode::Active {
                mem_idx,
                offset_expr,
            };
            Ok((
                WasmData {
                    mode,
                    bytes: data_bytes,
                },
                bytes,
            ))
        }
        _ => Err(WasmDecodeError::InvalidData),
    }
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
            decode_global_section(section, wmod)?;
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
            decode_element_section(section, wmod)?;
            Ok(((), rest))
        }
        SectionId::Data => {
            decode_data_section(section, wmod)?;
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

fn decode_magic_bytes(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, ()>> {
    let (buf, bytes) = take_bytes::<4>(bytes)?;
    match buf {
        [0, b'a', b's', b'm'] => Ok(((), bytes)),
        _ => Err(WasmDecodeError::MagicBytes),
    }
}

fn decode_version(bytes: &[u8]) -> WasmDecodeResult<Decoded<'_, u32>> {
    let (buf, bytes) = take_bytes::<4>(bytes)?;
    Ok((u32::from_le_bytes(buf), bytes))
}

pub fn decode(bytes: &[u8]) -> WasmDecodeResult<WasmModuleRaw> {
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
