
#include "wrun.h"

void wrun_num_default(WasmNumType numtype, WasmNumValue *num) {
    switch (numtype) {
        case WasmNumI32:
            num->i32 = 0;
            break;
        case WasmNumI64:
            num->i64 = 0;
            break;
        case WasmNumF32:
            num->f32 = 0;
            break;
        case WasmNumF64:
            num->f64 = 0;
            break;
    }
}

void wrun_ref_default(WasmRefType reftype, WasmRefValue *ref) {
    switch (reftype) {
        case WasmRefFunc:
            ref->funcaddr = 0;
            break;
        case WasmRefExtern:
            ref->externaddr = 0;
            break;
    }
}

void wrun_vec_default(WasmVecType vectype, WasmVecValue *vec) {
    switch (vectype) {
        case WasmVecV128:
            for (size_t i = 0; i < 8; i++) {
                *vec[i] = 0;
            }
            break;
    }
}

void wrun_value_default(WasmValueType valtype, WasmValue *value) {
    switch (valtype.kind) {
        case WasmValueTypeNum:
            return wrun_num_default(valtype.value.num, &value->num);
        case WasmValueTypeRef:
            return wrun_ref_default(valtype.value.ref, &value->ref);
        case WasmValueTypeVec:
            return wrun_vec_default(valtype.value.vec, &value->vec);
    }
}

void wrun_result_init(WasmResult *result) {
    vec_init(&result->values);
}

void wrun_store_init(WasmStore *store) {
    return;
}

WasmInstantiateResult wrun_instantiate(WasmInstantiateRequest req) {
    return WasmInstantiateOk;
}
