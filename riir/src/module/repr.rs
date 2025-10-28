pub type WasmModule = WasmModuleRepr<WasmInstruction>;
pub type WasmModuleRaw = WasmModuleRepr<WasmInstructionRaw>;

#[derive(Debug)]
pub struct WasmModuleRepr<TWasmInstruction> {
    pub version: u32,
    pub types: Box<[WasmFuncType]>,
    pub funcs: Box<[WasmFunc<TWasmInstruction>]>,
    pub tables: Box<[WasmTableType]>,
    pub mems: Box<[WasmMemType]>,
    pub globals: Box<[WasmGlobal<TWasmInstruction>]>,
    pub elems: Box<[WasmElem<TWasmInstruction>]>,
    pub datas: Box<[WasmData<TWasmInstruction>]>,
    pub start: Option<WasmFuncIdx>,
    pub imports: Box<[WasmImport]>,
    pub exports: Box<[WasmExport]>,
    pub customs: Box<[WasmCustom]>,
}

#[derive(Debug, Copy, Clone)]
pub struct WasmTypeIdx(pub u32);
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct WasmFuncIdx(pub u32);
#[derive(Debug, Copy, Clone)]
pub struct WasmTableIdx(pub u32);
#[derive(Debug, Copy, Clone)]
pub struct WasmMemIdx(pub u32);
#[derive(Debug, Copy, Clone)]
pub struct WasmGlobalIdx(pub u32);
#[derive(Debug, Copy, Clone)]
pub struct WasmElemIdx(pub u32);
#[derive(Debug, Copy, Clone)]
pub struct WasmDataIdx(pub u32);
#[derive(Debug, Copy, Clone)]
pub struct WasmLabelIdx(pub u32);
#[derive(Debug, Copy, Clone)]
pub struct WasmLocalIdx(pub u32);

impl WasmMemIdx {
    pub const ZERO: WasmMemIdx = WasmMemIdx(0);
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum WasmNumType {
    I32,
    I64,
    F32,
    F64,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum WasmVecType {
    V128,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum WasmRefType {
    FuncRef,
    ExternRef,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum WasmValueType {
    Num(WasmNumType),
    Vec(WasmVecType),
    Ref(WasmRefType),
}

macro_rules! t {
    (i32) => {
        crate::module::WasmValueType::Num(crate::module::WasmNumType::I32)
    };
    (i64) => {
        crate::module::WasmValueType::Num(crate::module::WasmNumType::I64)
    };
    (f32) => {
        crate::module::WasmValueType::Num(crate::module::WasmNumType::F32)
    };
    (f64) => {
        crate::module::WasmValueType::Num(crate::module::WasmNumType::F64)
    };
    (funcref) => {
        crate::module::WasmValueType::Ref(crate::module::WasmRefType::FuncRef)
    };
    (externref) => {
        crate::module::WasmValueType::Ref(crate::module::WasmRefType::ExternRef)
    };
    (v128) => {
        crate::module::WasmValueType::Vec(crate::module::WasmVecType::V128)
    };
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WasmResultType(pub Box<[WasmValueType]>);

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct WasmFuncType {
    pub input_type: WasmResultType,
    pub output_type: WasmResultType,
}

#[derive(Debug)]
pub enum WasmBlockType {
    TypeRef(WasmTypeIdx),
    InlineType(Option<WasmValueType>),
}

#[derive(Debug)]
pub struct WasmMemArg {
    pub offset: u32,
    pub align: u32,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
pub struct WasmInstructionIdx(pub u32);

pub trait Immediates: std::fmt::Debug {
    type BlockImmediates: std::fmt::Debug;
    type LoopImmediates: std::fmt::Debug;
    type IfImmediates: std::fmt::Debug;
}

#[derive(Debug)]
pub struct NoImmediates;

impl Immediates for NoImmediates {
    type BlockImmediates = ();
    type LoopImmediates = ();
    type IfImmediates = ();
}

#[derive(Debug)]
pub struct VerifiedImmediates;

#[derive(Debug)]
pub struct VerifiedIfImmediates {
    pub end_ic: WasmInstructionIdx,
    pub else_ic: Option<WasmInstructionIdx>,
}

impl Immediates for VerifiedImmediates {
    type BlockImmediates = WasmInstructionIdx;
    type LoopImmediates = WasmInstructionIdx;
    type IfImmediates = VerifiedIfImmediates;
}

pub type WasmInstruction = WasmInstructionRepr<VerifiedImmediates>;
pub type WasmInstructionRaw = WasmInstructionRepr<NoImmediates>;
pub type WasmExpr = [WasmInstruction];
pub type WasmExprRaw = [WasmInstructionRaw];

#[derive(Debug)]
pub enum WasmInstructionRepr<I: Immediates> {
    Unreachable,
    Nop,
    Block {
        block_type: WasmBlockType,
        imm: I::BlockImmediates,
    },
    Loop {
        block_type: WasmBlockType,
        imm: I::LoopImmediates,
    },
    If {
        block_type: WasmBlockType,
        imm: I::IfImmediates,
    },
    Else,
    Break {
        label_idx: WasmLabelIdx,
    },
    BreakIf {
        label_idx: WasmLabelIdx,
    },
    BreakTable {
        labels: Box<[WasmLabelIdx]>,
        default_label: WasmLabelIdx,
    },
    Return,
    Call {
        func_idx: WasmFuncIdx,
    },
    CallIndirect {
        table_idx: WasmTableIdx,
        type_idx: WasmTypeIdx,
    },
    ExprEnd,
    RefNull {
        ref_type: WasmRefType,
    },
    RefIsNull,
    RefFunc {
        func_idx: WasmFuncIdx,
    },
    Drop,
    Select {
        value_types: Box<[WasmValueType]>,
    },
    LocalGet {
        local_idx: WasmLocalIdx,
    },
    LocalSet {
        local_idx: WasmLocalIdx,
    },
    LocalTee {
        local_idx: WasmLocalIdx,
    },
    GlobalGet {
        global_idx: WasmGlobalIdx,
    },
    GlobalSet {
        global_idx: WasmGlobalIdx,
    },
    TableGet {
        table_idx: WasmTableIdx,
    },
    TableSet {
        table_idx: WasmTableIdx,
    },
    TableSize {
        table_idx: WasmTableIdx,
    },
    TableGrow {
        table_idx: WasmTableIdx,
    },
    TableFill {
        table_idx: WasmTableIdx,
    },
    TableCopy {
        dst: WasmTableIdx,
        src: WasmTableIdx,
    },
    TableInit {
        table_idx: WasmTableIdx,
        elem_idx: WasmElemIdx,
    },
    ElemDrop {
        elem_idx: WasmElemIdx,
    },
    I32Load {
        memarg: WasmMemArg,
    },
    I64Load {
        memarg: WasmMemArg,
    },
    F32Load {
        memarg: WasmMemArg,
    },
    F64Load {
        memarg: WasmMemArg,
    },
    I32Load8S {
        memarg: WasmMemArg,
    },
    I32Load8U {
        memarg: WasmMemArg,
    },
    I32Load16S {
        memarg: WasmMemArg,
    },
    I32Load16U {
        memarg: WasmMemArg,
    },
    I64Load8S {
        memarg: WasmMemArg,
    },
    I64Load8U {
        memarg: WasmMemArg,
    },
    I64Load16S {
        memarg: WasmMemArg,
    },
    I64Load16U {
        memarg: WasmMemArg,
    },
    I64Load32S {
        memarg: WasmMemArg,
    },
    I64Load32U {
        memarg: WasmMemArg,
    },
    I32Store {
        memarg: WasmMemArg,
    },
    I64Store {
        memarg: WasmMemArg,
    },
    F32Store {
        memarg: WasmMemArg,
    },
    F64Store {
        memarg: WasmMemArg,
    },
    I32Store8 {
        memarg: WasmMemArg,
    },
    I32Store16 {
        memarg: WasmMemArg,
    },
    I64Store8 {
        memarg: WasmMemArg,
    },
    I64Store16 {
        memarg: WasmMemArg,
    },
    I64Store32 {
        memarg: WasmMemArg,
    },
    MemorySize,
    MemoryGrow,
    MemoryInit {
        data_idx: WasmDataIdx,
    },
    DataDrop {
        data_idx: WasmDataIdx,
    },
    MemoryCopy,
    MemoryFill,

    I32Const {
        val: i32,
    },
    I64Const {
        val: i64,
    },
    F32Const {
        val: f32,
    },
    F64Const {
        val: f64,
    },

    I32EqZ,
    I32Eq,
    I32Neq,
    I32LtS,
    I32LtU,
    I32GtS,
    I32GtU,
    I32LeS,
    I32LeU,
    I32GeS,
    I32GeU,

    I64EqZ,
    I64Eq,
    I64Neq,
    I64LtS,
    I64LtU,
    I64GtS,
    I64GtU,
    I64LeS,
    I64LeU,
    I64GeS,
    I64GeU,

    F32Eq,
    F32Neq,
    F32Lt,
    F32Gt,
    F32Le,
    F32Ge,

    F64Eq,
    F64Neq,
    F64Lt,
    F64Gt,
    F64Le,
    F64Ge,

    I32Clz,
    I32Ctz,
    I32Popcnt,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I32Rotl,
    I32Rotr,

    I64Clz,
    I64Ctz,
    I64Popcnt,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    I64Rotl,
    I64Rotr,

    F32Abs,
    F32Neg,
    F32Ceil,
    F32Floor,
    F32Trunc,
    F32Nearest,
    F32Sqrt,
    F32Add,
    F32Sub,
    F32Mul,
    F32Div,
    F32Min,
    F32Max,
    F32CopySign,

    F64Abs,
    F64Neg,
    F64Ceil,
    F64Floor,
    F64Trunc,
    F64Nearest,
    F64Sqrt,
    F64Add,
    F64Sub,
    F64Mul,
    F64Div,
    F64Min,
    F64Max,
    F64CopySign,

    I32WrapI64,
    I32TruncF32S,
    I32TruncF32U,
    I32TruncF64S,
    I32TruncF64U,
    I64ExtendI32S,
    I64ExtendI32U,
    I64TruncF32S,
    I64TruncF32U,
    I64TruncF64S,
    I64TruncF64U,
    F32ConvertI32S,
    F32ConvertI32U,
    F32ConvertI64S,
    F32ConvertI64U,
    F32DemoteF64,
    F64ConvertI32S,
    F64ConvertI32U,
    F64ConvertI64S,
    F64ConvertI64U,
    F64PromoteF32,
    I32ReinterpretF32,
    I64ReinterpretF64,
    F32ReinterpretI32,
    F64ReinterpretI64,

    I32Extend8S,
    I32Extend16S,
    I64Extend8S,
    I64Extend16S,
    I64Extend32S,

    I32TruncSatF32S,
    I32TruncSatF32U,
    I32TruncSatF64S,
    I32TruncSatF64U,
    I64TruncSatF32S,
    I64TruncSatF32U,
    I64TruncSatF64S,
    I64TruncSatF64U,
}

#[derive(Debug)]
pub struct WasmFunc<TWasmInstruction = WasmInstruction> {
    pub type_idx: WasmTypeIdx,
    pub locals: Box<[WasmValueType]>,
    pub body: Box<[TWasmInstruction]>,
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

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub enum WasmGlobalMutability {
    Mutable,
    Immutable,
}

#[derive(Debug, Eq, PartialEq, Copy, Clone)]
pub struct WasmGlobalType {
    pub mutability: WasmGlobalMutability,
    pub val_type: WasmValueType,
}

#[derive(Debug)]
pub struct WasmGlobal<TWasmInstruction = WasmInstruction> {
    pub global_type: WasmGlobalType,
    pub init: Box<[TWasmInstruction]>,
}

#[derive(Debug)]
pub enum WasmElemMode<TWasmInstruction = WasmInstruction> {
    Passive,
    Active {
        table_idx: WasmTableIdx,
        offset_expr: Box<[TWasmInstruction]>,
    },
    Declarative,
}

#[derive(Debug)]
pub struct WasmElem<TWasmInstruction = WasmInstruction> {
    pub ref_type: WasmRefType,
    pub init: Box<[Box<[TWasmInstruction]>]>,
    pub elem_mode: WasmElemMode<TWasmInstruction>,
}

#[derive(Debug)]
pub enum WasmDataMode<TWasmInstruction = WasmInstruction> {
    Passive,
    Active {
        mem_idx: WasmMemIdx,
        offset_expr: Box<[TWasmInstruction]>,
    },
}

#[derive(Debug)]
pub struct WasmData<TWasmInstruction = WasmInstruction> {
    pub bytes: Box<[u8]>,
    pub mode: WasmDataMode<TWasmInstruction>,
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
