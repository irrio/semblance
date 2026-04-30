use crate::{
    inst::table::WasmFuncAddr,
    module::{WasmNumType, WasmRefType, WasmValueType},
};

#[derive(Clone, Copy)]
pub union WasmValue {
    pub num: WasmNumValue,
    pub vec: WasmVecValue,
    pub ref_: WasmRefValue,
}

impl WasmValue {
    pub fn default_of_type(value_type: &WasmValueType) -> Self {
        use WasmValueType::*;
        match value_type {
            Num(WasmNumType::F32) => 0f32.into(),
            Num(WasmNumType::F64) => 0f64.into(),
            Num(WasmNumType::I32) => 0i32.into(),
            Num(WasmNumType::I64) => 0i64.into(),
            Ref(_) => WasmRefValue::NULL.into(),
            Vec(_) => WasmValue { vec: 0 },
        }
    }
}

impl Into<WasmValue> for i32 {
    fn into(self) -> WasmValue {
        WasmValue {
            num: WasmNumValue { i32: self },
        }
    }
}

impl Into<WasmValue> for i64 {
    fn into(self) -> WasmValue {
        WasmValue {
            num: WasmNumValue { i64: self },
        }
    }
}

impl Into<WasmValue> for f32 {
    fn into(self) -> WasmValue {
        WasmValue {
            num: WasmNumValue { f32: self },
        }
    }
}

impl Into<WasmValue> for f64 {
    fn into(self) -> WasmValue {
        WasmValue {
            num: WasmNumValue { f64: self },
        }
    }
}

#[derive(Clone, Copy)]
pub union WasmNumValue {
    pub i32: i32,
    pub i64: i64,
    pub f32: f32,
    pub f64: f64,
}

pub type WasmVecValue = i128;

#[derive(Clone, Copy)]
pub union WasmRefValue {
    pub func: WasmFuncAddr,
    pub extern_: WasmExternAddr,
}

#[derive(Debug, Copy, Clone)]
pub struct WasmExternAddr(pub u32);

impl Into<WasmValue> for WasmRefValue {
    fn into(self) -> WasmValue {
        WasmValue { ref_: self }
    }
}

impl WasmRefValue {
    pub const NULL: WasmRefValue = WasmRefValue {
        func: WasmFuncAddr::NULL,
    };
}

pub fn wasm_value_eq(ty: &WasmValueType, v1: &WasmValue, v2: &WasmValue) -> bool {
    match ty {
        WasmValueType::Num(WasmNumType::I32) => unsafe { v1.num.i32 == v2.num.i32 },
        WasmValueType::Num(WasmNumType::I64) => unsafe { v1.num.i64 == v2.num.i64 },
        WasmValueType::Num(WasmNumType::F32) => unsafe {
            (v1.num.f32.is_nan() && v2.num.f32.is_nan()) || v1.num.f32 == v2.num.f32
        },
        WasmValueType::Num(WasmNumType::F64) => unsafe {
            (v1.num.f64.is_nan() && v2.num.f64.is_nan()) || v1.num.f64 == v2.num.f64
        },
        WasmValueType::Ref(WasmRefType::ExternRef) => unsafe {
            v1.ref_.extern_.0 == v2.ref_.extern_.0
        },
        WasmValueType::Ref(WasmRefType::FuncRef) => unsafe { v1.ref_.func == v2.ref_.func },
        WasmValueType::Vec(_wasm_vec_type) => todo!(),
    }
}
