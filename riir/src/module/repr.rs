#[derive(Debug)]
pub struct WasmModule {
    pub version: u32,
    pub types: Box<[WasmFuncType]>,
    pub funcs: Box<[WasmFunc]>,
    pub tables: Box<[WasmTableType]>,
    pub mems: Box<[WasmMemType]>,
    pub globals: Box<[WasmGlobal]>,
    pub elems: Box<[WasmElem]>,
    pub datas: Box<[WasmData]>,
    pub start: Option<WasmFuncIdx>,
    pub imports: Box<[WasmImport]>,
    pub exports: Box<[WasmExport]>,
    pub customs: Box<[WasmCustom]>,
}

#[derive(Debug)]
pub struct WasmTypeIdx(pub u32);
#[derive(Debug)]
pub struct WasmFuncIdx(pub u32);
#[derive(Debug)]
pub struct WasmTableIdx(pub u32);
#[derive(Debug)]
pub struct WasmMemIdx(pub u32);
#[derive(Debug)]
pub struct WasmGlobalIdx(pub u32);
#[derive(Debug)]
pub struct WasmElemIdx(pub u32);
#[derive(Debug)]
pub struct WasmDataIdx(pub u32);

#[derive(Debug)]
pub enum WasmNumType {
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug)]
pub enum WasmVecType {
    V128,
}

#[derive(Debug)]
pub enum WasmRefType {
    FuncRef,
    ExternRef,
}

#[derive(Debug)]
pub enum WasmValueType {
    Num(WasmNumType),
    Vec(WasmVecType),
    Ref(WasmRefType),
}

#[derive(Debug)]
pub struct WasmResultType(pub Box<[WasmValueType]>);

#[derive(Debug)]
pub struct WasmFuncType {
    pub input_type: WasmResultType,
    pub output_type: WasmResultType,
}

#[derive(Debug)]
pub struct WasmInstruction {
    // ...
}

#[derive(Debug)]
pub struct WasmExpr(pub Box<[WasmInstruction]>);

#[derive(Debug)]
pub struct WasmFunc {
    pub type_idx: WasmTypeIdx,
    pub locals: Box<[WasmValueType]>,
    pub body: WasmExpr,
}

#[derive(Debug)]
pub struct WasmLimits {
    pub min: u32,
    pub max: Option<u32>,
}

#[derive(Debug)]
pub struct WasmTableType {
    pub limits: WasmLimits,
    pub ref_type: WasmRefType,
}

#[derive(Debug)]
pub struct WasmMemType {
    pub limits: WasmLimits,
}

#[derive(Debug)]
pub enum WasmGlobalMutability {
    Mutable,
    Immutable,
}

#[derive(Debug)]
pub struct WasmGlobalType {
    pub mutability: WasmGlobalMutability,
    pub val_type: WasmValueType,
}

#[derive(Debug)]
pub struct WasmGlobal {
    pub global_type: WasmGlobalType,
    pub init: WasmExpr,
}

#[derive(Debug)]
pub struct WasmActiveElemParams {
    pub table_idx: WasmTableIdx,
    pub offset_expr: WasmExpr,
}

#[derive(Debug)]
pub enum WasmElemMode {
    Passive,
    Active(WasmActiveElemParams),
    Declarative,
}

#[derive(Debug)]
pub struct WasmElem {
    pub ref_type: WasmRefType,
    pub init: Box<[WasmExpr]>,
    pub elem_mode: WasmElemMode,
}

#[derive(Debug)]
pub struct WasmActiveDataParams {
    pub mem_idx: WasmMemIdx,
    pub offset_expr: WasmExpr,
}

#[derive(Debug)]
pub enum WasmDataMode {
    Passive,
    Active(WasmActiveDataParams),
}

#[derive(Debug)]
pub struct WasmData {
    pub bytes: Box<[u8]>,
    pub mode: WasmDataMode,
}

#[derive(Debug)]
pub enum WasmImportDesc {
    Func(WasmTypeIdx),
    Table(WasmTableType),
    Mem(WasmMemType),
    Global(WasmGlobalType),
}

#[derive(Debug)]
pub struct WasmName(pub Box<str>);

#[derive(Debug)]
pub struct WasmImport {
    pub module_name: WasmName,
    pub item_name: WasmName,
    pub desc: WasmImportDesc,
}

#[derive(Debug)]
pub enum WasmExportDesc {
    Func(WasmFuncIdx),
    Table(WasmTableIdx),
    Mem(WasmMemIdx),
    Global(WasmGlobalIdx),
}

#[derive(Debug)]
pub struct WasmExport {
    pub name: WasmName,
    pub desc: WasmExportDesc,
}

pub struct WasmCustom {
    pub name: WasmName,
    pub bytes: Box<[u8]>,
}

impl std::fmt::Debug for WasmCustom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "WasmCustom({:?}, [{} bytes])",
            self.name,
            self.bytes.len()
        )
    }
}
