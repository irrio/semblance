use super::*;

#[derive(Default)]
pub struct WasmModuleBuilder {
    version: u32,
    datacount: Option<u32>,
    types: Vec<WasmFuncType>,
    funcs: Vec<WasmTypeIdx>,
    code: Vec<WasmCode>,
    tables: Vec<WasmTableType>,
    mems: Vec<WasmMemType>,
    globals: Vec<WasmGlobal>,
    elems: Vec<WasmElem>,
    datas: Vec<WasmData>,
    start: Option<WasmFuncIdx>,
    imports: Vec<WasmImport>,
    exports: Vec<WasmExport>,
    customs: Vec<WasmCustom>,
}

pub struct WasmCode {
    pub locals: Box<[WasmValueType]>,
    pub body: WasmExpr,
}

impl WasmModuleBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn version(&mut self, version: u32) {
        self.version = version;
    }

    pub fn datacount(&mut self, count: u32) {
        self.datacount = Some(count);
    }

    pub fn reserve_types(&mut self, num: usize) {
        self.types.reserve_exact(num);
    }

    pub fn push_type(&mut self, type_: WasmFuncType) {
        self.types.push(type_);
    }

    pub fn reserve_funcs(&mut self, num: usize) {
        self.funcs.reserve_exact(num);
    }

    pub fn push_func(&mut self, type_idx: WasmTypeIdx) {
        self.funcs.push(type_idx);
    }

    pub fn reserve_code(&mut self, num: usize) {
        self.code.reserve_exact(num);
    }

    pub fn push_code(&mut self, code: WasmCode) {
        self.code.push(code);
    }

    pub fn reserve_tables(&mut self, num: usize) {
        self.tables.reserve_exact(num);
    }

    pub fn push_table(&mut self, table: WasmTableType) {
        self.tables.push(table);
    }

    pub fn reserve_mems(&mut self, num: usize) {
        self.mems.reserve_exact(num);
    }

    pub fn push_mem(&mut self, mem: WasmMemType) {
        self.mems.push(mem);
    }

    pub fn reserve_globals(&mut self, num: usize) {
        self.globals.reserve_exact(num);
    }

    pub fn push_global(&mut self, global: WasmGlobal) {
        self.globals.push(global);
    }

    pub fn reserve_elems(&mut self, num: usize) {
        self.elems.reserve_exact(num);
    }

    pub fn push_elem(&mut self, elem: WasmElem) {
        self.elems.push(elem);
    }

    pub fn reserve_datas(&mut self, num: usize) {
        self.datas.reserve_exact(num);
    }

    pub fn push_data(&mut self, data: WasmData) {
        self.datas.push(data);
    }

    pub fn start(&mut self, func_idx: WasmFuncIdx) {
        self.start = Some(func_idx);
    }

    pub fn reserve_imports(&mut self, num: usize) {
        self.imports.reserve_exact(num);
    }

    pub fn push_import(&mut self, import: WasmImport) {
        self.imports.push(import);
    }

    pub fn reserve_exports(&mut self, num: usize) {
        self.exports.reserve_exact(num);
    }

    pub fn push_export(&mut self, export: WasmExport) {
        self.exports.push(export);
    }

    pub fn reserve_custom(&mut self, num: usize) {
        self.customs.reserve_exact(num);
    }

    pub fn push_custom(&mut self, custom: WasmCustom) {
        self.customs.push(custom);
    }

    pub fn build(self) -> WasmModule {
        let funcs = self
            .funcs
            .into_iter()
            .zip(self.code.into_iter())
            .map(|(type_idx, code)| WasmFunc {
                type_idx,
                locals: code.locals,
                body: code.body,
            })
            .collect::<Vec<_>>();
        WasmModule {
            version: self.version,
            types: self.types.into_boxed_slice(),
            funcs: funcs.into_boxed_slice(),
            tables: self.tables.into_boxed_slice(),
            mems: self.mems.into_boxed_slice(),
            globals: self.globals.into_boxed_slice(),
            elems: self.elems.into_boxed_slice(),
            datas: self.datas.into_boxed_slice(),
            start: self.start,
            imports: self.imports.into_boxed_slice(),
            exports: self.exports.into_boxed_slice(),
            customs: self.customs.into_boxed_slice(),
        }
    }
}

impl Into<WasmModule> for WasmModuleBuilder {
    fn into(self) -> WasmModule {
        self.build()
    }
}

#[derive(Default)]
pub struct WasmResultTypeBuilder(Vec<WasmValueType>);

impl WasmResultTypeBuilder {
    pub fn new() -> Self {
        WasmResultTypeBuilder::default()
    }

    pub fn reserve(&mut self, n: usize) {
        self.0.reserve_exact(n);
    }

    pub fn push_value_type(&mut self, vtype: WasmValueType) {
        self.0.push(vtype);
    }

    pub fn build(self) -> WasmResultType {
        WasmResultType(self.0.into_boxed_slice())
    }
}

impl Into<WasmResultType> for WasmResultTypeBuilder {
    fn into(self) -> WasmResultType {
        self.build()
    }
}

#[derive(Default)]
pub struct WasmExprBuilder(Vec<WasmInstruction>);

impl WasmExprBuilder {
    pub fn new() -> Self {
        WasmExprBuilder::default()
    }

    pub fn push_instr(&mut self, instr: WasmInstruction) -> &WasmInstruction {
        self.0.push(instr);
        self.0.last().unwrap()
    }

    pub fn build(self) -> WasmExpr {
        WasmExpr(self.0.into_boxed_slice())
    }
}

impl Into<WasmExpr> for WasmExprBuilder {
    fn into(self) -> WasmExpr {
        self.build()
    }
}
