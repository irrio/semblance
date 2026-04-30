use std::fmt::Display;

use crate::{
    inst::val::{WasmValue, wasm_value_eq},
    module::{WasmNumType, WasmRefType, WasmValueType},
};

pub struct WasmResult(pub Vec<WasmValue>);

pub struct DynamicWasmResult {
    pub ty: Box<[WasmValueType]>,
    pub res: WasmResult,
}

impl DynamicWasmResult {
    pub fn void() -> Self {
        DynamicWasmResult {
            ty: Box::new([]),
            res: WasmResult(vec![]),
        }
    }
}

impl std::fmt::Debug for DynamicWasmResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}

impl Display for DynamicWasmResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.ty.len() != 1 {
            write!(f, "(")?;
        }
        for (i, (ty, val)) in self.ty.iter().zip(self.res.0.iter()).enumerate() {
            match ty {
                WasmValueType::Num(numt) => match numt {
                    WasmNumType::I32 => write!(f, "{}", unsafe { val.num.i32 })?,
                    WasmNumType::I64 => write!(f, "{}", unsafe { val.num.i64 })?,
                    WasmNumType::F32 => write!(f, "{}", unsafe { val.num.f32 })?,
                    WasmNumType::F64 => write!(f, "{}", unsafe { val.num.f64 })?,
                },
                WasmValueType::Vec(_vect) => {
                    todo!()
                }
                WasmValueType::Ref(reft) => match reft {
                    WasmRefType::ExternRef => write!(f, "{}", unsafe { val.ref_.extern_.0 })?,
                    WasmRefType::FuncRef => write!(f, "{}", unsafe { val.ref_.func })?,
                },
            }
            if i < self.ty.len() - 1 {
                write!(f, ", ")?;
            }
        }
        if self.ty.len() != 1 {
            write!(f, ")")?;
        }
        Ok(())
    }
}

impl PartialEq for DynamicWasmResult {
    fn eq(&self, other: &Self) -> bool {
        if self.ty != other.ty {
            return false;
        }
        for ((v1, v2), ty) in self
            .res
            .0
            .iter()
            .zip(other.res.0.iter())
            .zip(self.ty.iter())
        {
            if !wasm_value_eq(ty, v1, v2) {
                return false;
            }
        }
        true
    }
}
