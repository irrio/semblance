use crate::{
    inst::{WasmFrame, WasmFuncImpl, WasmRefValue, WasmStack, WasmStore, WasmTrap},
    module::{WasmInstruction, WasmMemIdx},
};

pub fn exec(
    stack: &mut WasmStack,
    store: &mut WasmStore,
    expr: &[WasmInstruction],
) -> Result<(), WasmTrap> {
    let mut ic = 0;
    loop {
        use WasmInstruction::*;
        match &expr[ic] {
            I32Const { val } => stack.push_value(*val),
            I64Const { val } => stack.push_value(*val),
            F32Const { val } => stack.push_value(*val),
            F64Const { val } => stack.push_value(*val),
            I32EqZ => {
                let a = stack.pop_value();
                stack.push_value((unsafe { a.num.i32 } == 0) as i32);
            }
            I32Eq => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 == b.num.i32 } as i32);
            }
            I32Neq => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 != b.num.i32 } as i32);
            }
            I32LtS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 < b.num.i32 } as i32);
            }
            I32LtU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) < (b.num.i32 as u32) } as i32);
            }
            I32GtS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 > b.num.i32 } as i32);
            }
            I32GtU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) > (b.num.i32 as u32) } as i32);
            }
            I32LeS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 <= b.num.i32 } as i32);
            }
            I32LeU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) <= (b.num.i32 as u32) } as i32);
            }
            I32GeS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 >= b.num.i32 } as i32);
            }
            I32GeU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) >= (b.num.i32 as u32) } as i32);
            }
            I32Clz => {
                let a = stack.pop_value();
                let clz = unsafe { a.num.i32 }.leading_zeros();
                stack.push_value(clz as i32);
            }
            I32Ctz => {
                let a = stack.pop_value();
                let ctz = unsafe { a.num.i32 }.trailing_zeros();
                stack.push_value(ctz as i32);
            }
            I32Popcnt => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 }.count_ones() as i32);
            }
            I32Add => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32.wrapping_add(b.num.i32) });
            }
            I32Sub => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32.wrapping_sub(b.num.i32) });
            }
            I32Mul => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32.wrapping_mul(b.num.i32) });
            }
            I32DivS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 / b.num.i32 });
            }
            I32DivU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) / (b.num.i32 as u32) } as i32);
            }
            I32RemS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 % b.num.i32 });
            }
            I32RemU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) % (b.num.i32 as u32) } as i32);
            }
            I32And => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 & b.num.i32 });
            }
            I32Or => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 | b.num.i32 });
            }
            I32Xor => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 ^ b.num.i32 });
            }
            I32Shl => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 << b.num.i32 });
            }
            I32ShrS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 >> b.num.i32 });
            }
            I32ShrU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) >> (b.num.i32 as u32) } as i32);
            }
            I32Rotl => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32.rotate_left(b.num.i32 as u32) });
            }
            I32Rotr => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32.rotate_right(b.num.i32 as u32) });
            }
            I64EqZ => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 == 0 } as i32);
            }
            I64Eq => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 == b.num.i64 } as i32);
            }
            I64Neq => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 != b.num.i64 } as i32);
            }
            I64LtS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 < b.num.i64 } as i32);
            }
            I64LtU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) < (b.num.i64 as u64) } as i32);
            }
            I64GtS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 > b.num.i64 } as i32);
            }
            I64GtU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) > (b.num.i64 as u64) } as i32);
            }
            I64LeS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 <= b.num.i64 } as i32);
            }
            I64LeU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) <= (b.num.i64 as u64) } as i32);
            }
            I64GeS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 >= b.num.i64 } as i32);
            }
            I64GeU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) >= (b.num.i64 as u64) } as i32);
            }
            I64Clz => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 }.leading_zeros() as i64);
            }
            I64Ctz => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 }.trailing_zeros() as i64);
            }
            I64Popcnt => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 }.count_ones() as i64);
            }
            I64Add => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64.wrapping_add(b.num.i64) });
            }
            I64Sub => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64.wrapping_sub(b.num.i64) });
            }
            I64Mul => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64.wrapping_mul(b.num.i64) });
            }
            I64DivS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 / b.num.i64 });
            }
            I64DivU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) / (b.num.i64 as u64) } as i64);
            }
            I64RemS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 % b.num.i64 });
            }
            I64RemU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) % (b.num.i64 as u64) } as i64);
            }
            I64And => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 & b.num.i64 });
            }
            I64Or => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 | b.num.i64 });
            }
            I64Xor => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 ^ b.num.i64 });
            }
            I64Shl => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 << b.num.i64 });
            }
            I64ShrS => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 >> b.num.i64 });
            }
            I64ShrU => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) >> (b.num.i64 as u64) } as i64);
            }
            I64Rotl => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64.rotate_left(b.num.i64 as u32) });
            }
            I64Rotr => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i64.rotate_right(b.num.i64 as u32) });
            }
            F32Eq => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 == b.num.f32 } as i32);
            }
            F32Neq => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 != b.num.f32 } as i32);
            }
            F32Lt => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 < b.num.f32 } as i32);
            }
            F32Gt => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 > b.num.f32 } as i32);
            }
            F32Le => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 <= b.num.f32 } as i32);
            }
            F32Ge => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 >= b.num.f32 } as i32);
            }
            F32Abs => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 }.abs());
            }
            F32Neg => {
                let a = stack.pop_value();
                stack.push_value(unsafe { -a.num.f32 });
            }
            F32Ceil => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 }.ceil());
            }
            F32Floor => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 }.floor());
            }
            F32Trunc => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 }.trunc());
            }
            F32Nearest => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 }.round_ties_even());
            }
            F32Sqrt => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 }.sqrt());
            }
            F32Add => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 + b.num.f32 });
            }
            F32Sub => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 - b.num.f32 });
            }
            F32Mul => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 * b.num.f32 });
            }
            F32Div => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 / b.num.f32 });
            }
            F32Min => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32.min(b.num.f32) });
            }
            F32Max => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32.max(b.num.f32) });
            }
            F32CopySign => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f32.copysign(b.num.f32) });
            }
            F64Eq => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 == b.num.f64 } as i32);
            }
            F64Neq => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 != b.num.f64 } as i32);
            }
            F64Lt => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 < b.num.f64 } as i32);
            }
            F64Gt => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 > b.num.f64 } as i32);
            }
            F64Le => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 <= b.num.f64 } as i32);
            }
            F64Ge => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 >= b.num.f64 } as i32);
            }
            F64Abs => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 }.abs());
            }
            F64Neg => {
                let a = stack.pop_value();
                stack.push_value(unsafe { -a.num.f64 });
            }
            F64Ceil => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 }.ceil());
            }
            F64Floor => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 }.floor());
            }
            F64Trunc => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 }.trunc());
            }
            F64Nearest => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 }.round_ties_even());
            }
            F64Sqrt => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 }.sqrt());
            }
            F64Add => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 + b.num.f64 });
            }
            F64Sub => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 - b.num.f64 });
            }
            F64Mul => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 * b.num.f64 });
            }
            F64Div => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 / b.num.f64 });
            }
            F64Min => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64.min(b.num.f64) });
            }
            F64Max => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64.max(b.num.f64) });
            }
            F64CopySign => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.f64.copysign(b.num.f64) });
            }
            TableInit {
                table_idx,
                elem_idx,
            } => todo!(),
            MemoryInit { data_idx } => {
                let n = unsafe { stack.pop_value().num.i32 } as usize;
                let s = unsafe { stack.pop_value().num.i32 } as usize;
                let d = unsafe { stack.pop_value().num.i32 } as usize;
                let frame = stack.current_frame();
                let winst = store.instances.resolve(frame.winst_id);
                let mem = store.mems.resolve_mut(winst.addr_of(WasmMemIdx::ZERO));
                let data = store.datas.resolve(winst.addr_of(*data_idx));
                let data_bytes = data.data.expect("use of dropped data");
                (&mut mem.data[d..(d + n)]).copy_from_slice(&data_bytes[s..(s + n)]);
            }
            ElemDrop { elem_idx } => {
                let frame = stack.current_frame();
                let elemaddr = store.instances.resolve(frame.winst_id).addr_of(*elem_idx);
                store.elems.resolve_mut(elemaddr).elem = Box::new([]);
            }
            DataDrop { data_idx } => {
                let frame = stack.current_frame();
                let dataaddr = store.instances.resolve(frame.winst_id).addr_of(*data_idx);
                store.datas.resolve_mut(dataaddr).data = None;
            }
            RefNull { ref_type: _ } => {
                stack.push_value(WasmRefValue::NULL);
            }
            RefFunc { func_idx } => {
                let frame = stack.current_frame();
                let funcaddr = store.instances.resolve(frame.winst_id).addr_of(*func_idx);
                stack.push_value(WasmRefValue { func: funcaddr });
            }
            GlobalGet { global_idx } => {
                let frame = stack.current_frame();
                let globaladdr = store.instances.resolve(frame.winst_id).addr_of(*global_idx);
                stack.push_value(store.globals.resolve(globaladdr).val);
            }
            LocalGet { local_idx } => {
                let frame = stack.current_frame();
                let val = frame.locals[local_idx.0 as usize];
                stack.push_value(val);
            }
            Call { func_idx } => {
                let winst_id = stack.current_frame().winst_id;
                let funcaddr = store.instances.resolve(winst_id).addr_of(*func_idx);
                let func = store.funcs.resolve(funcaddr);
                let args = stack.pop_values(func.type_.input_type.0.len());
                match func.impl_ {
                    WasmFuncImpl::Host { hostfunc } => {
                        let ret = hostfunc(store, winst_id, &args);
                        for val in ret {
                            stack.push_value(val);
                        }
                    }
                    WasmFuncImpl::Wasm {
                        winst_id,
                        func: _funcimpl,
                    } => {
                        stack.push_frame(WasmFrame {
                            arity: func.type_.output_type.0.len() as u32,
                            locals: Box::new([]),
                            winst_id,
                        });
                        todo!();
                    }
                }
            }
            Unreachable => return Err(WasmTrap {}),
            ExprEnd => break,
            instr @ _ => panic!("instr unimplemented: {:?}", instr),
        }
        ic += 1;
    }
    Ok(())
}
