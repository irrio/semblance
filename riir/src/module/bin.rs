use std::string::FromUtf8Error;

use super::{
    builder::{WasmCode, WasmModuleBuilder, WasmResultTypeBuilder},
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

fn decode_expr(bytes: &[u8]) -> WasmDecodeResult<Decoded<WasmExpr>> {
    todo!();
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
        SectionId::Code => {
            decode_code_section(section, wmod)?;
            Ok(((), rest))
        }
        _ => {
            eprintln!("Skipping {:?}", sid);
            Ok(((), rest))
        } //SectionId::Element => todo!(),
          //SectionId::Data => todo!(),
          //SectionId::DataCount => todo!(),
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
