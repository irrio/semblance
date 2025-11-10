use crate::{
    inst::{
        ControlStackEntry, WasmFrame, WasmFuncImpl, WasmLabel, WasmRefValue, WasmStack, WasmStore,
        WasmTrap, WasmValue,
    },
    module::{WasmExpr, WasmInstruction, WasmInstructionRepr, WasmLabelIdx, WasmMemIdx},
};

pub fn exec<'wmod>(
    stack: &mut WasmStack,
    store: &mut WasmStore<'wmod>,
    expr: &WasmExpr,
) -> Result<(), WasmTrap> {
    let mut ip: *const WasmInstruction = &expr[0];
    loop {
        use WasmInstructionRepr::*;
        match unsafe { &*ip } {
            I32Const { val } => stack.push_value(*val),
            I64Const { val } => stack.push_value(*val),
            F32Const { val } => stack.push_value(*val),
            F64Const { val } => stack.push_value(*val),
            I32EqZ => {
                let a = stack.pop_value();
                stack.push_value((unsafe { a.num.i32 } == 0) as i32);
            }
            I32Eq => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 == b.num.i32 } as i32);
            }
            I32Neq => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 != b.num.i32 } as i32);
            }
            I32LtS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 < b.num.i32 } as i32);
            }
            I32LtU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) < (b.num.i32 as u32) } as i32);
            }
            I32GtS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 > b.num.i32 } as i32);
            }
            I32GtU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) > (b.num.i32 as u32) } as i32);
            }
            I32LeS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 <= b.num.i32 } as i32);
            }
            I32LeU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) <= (b.num.i32 as u32) } as i32);
            }
            I32GeS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 >= b.num.i32 } as i32);
            }
            I32GeU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
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
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32.wrapping_add(b.num.i32) });
            }
            I32Sub => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                if (unsafe { a.num.i32 } <= 0) {
                    todo!("debug me");
                }
                stack.push_value(unsafe { a.num.i32.wrapping_sub(b.num.i32) });
            }
            I32Mul => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32.wrapping_mul(b.num.i32) });
            }
            I32DivS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 / b.num.i32 });
            }
            I32DivU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) / (b.num.i32 as u32) } as i32);
            }
            I32RemS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 % b.num.i32 });
            }
            I32RemU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) % (b.num.i32 as u32) } as i32);
            }
            I32And => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 & b.num.i32 });
            }
            I32Or => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 | b.num.i32 });
            }
            I32Xor => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 ^ b.num.i32 });
            }
            I32Shl => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 << b.num.i32 });
            }
            I32ShrS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 >> b.num.i32 });
            }
            I32ShrU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i32 as u32) >> (b.num.i32 as u32) } as i32);
            }
            I32Rotl => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32.rotate_left(b.num.i32 as u32) });
            }
            I32Rotr => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i32.rotate_right(b.num.i32 as u32) });
            }
            I64EqZ => {
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 == 0 } as i32);
            }
            I64Eq => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 == b.num.i64 } as i32);
            }
            I64Neq => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 != b.num.i64 } as i32);
            }
            I64LtS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 < b.num.i64 } as i32);
            }
            I64LtU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) < (b.num.i64 as u64) } as i32);
            }
            I64GtS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 > b.num.i64 } as i32);
            }
            I64GtU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) > (b.num.i64 as u64) } as i32);
            }
            I64LeS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 <= b.num.i64 } as i32);
            }
            I64LeU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) <= (b.num.i64 as u64) } as i32);
            }
            I64GeS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 >= b.num.i64 } as i32);
            }
            I64GeU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
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
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64.wrapping_add(b.num.i64) });
            }
            I64Sub => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64.wrapping_sub(b.num.i64) });
            }
            I64Mul => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64.wrapping_mul(b.num.i64) });
            }
            I64DivS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 / b.num.i64 });
            }
            I64DivU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) / (b.num.i64 as u64) } as i64);
            }
            I64RemS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 % b.num.i64 });
            }
            I64RemU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) % (b.num.i64 as u64) } as i64);
            }
            I64And => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 & b.num.i64 });
            }
            I64Or => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 | b.num.i64 });
            }
            I64Xor => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 ^ b.num.i64 });
            }
            I64Shl => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 << b.num.i64 });
            }
            I64ShrS => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64 >> b.num.i64 });
            }
            I64ShrU => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { (a.num.i64 as u64) >> (b.num.i64 as u64) } as i64);
            }
            I64Rotl => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64.rotate_left(b.num.i64 as u32) });
            }
            I64Rotr => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.i64.rotate_right(b.num.i64 as u32) });
            }
            F32Eq => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 == b.num.f32 } as i32);
            }
            F32Neq => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 != b.num.f32 } as i32);
            }
            F32Lt => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 < b.num.f32 } as i32);
            }
            F32Gt => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 > b.num.f32 } as i32);
            }
            F32Le => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 <= b.num.f32 } as i32);
            }
            F32Ge => {
                let b = stack.pop_value();
                let a = stack.pop_value();
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
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 + b.num.f32 });
            }
            F32Sub => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 - b.num.f32 });
            }
            F32Mul => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 * b.num.f32 });
            }
            F32Div => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32 / b.num.f32 });
            }
            F32Min => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32.min(b.num.f32) });
            }
            F32Max => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32.max(b.num.f32) });
            }
            F32CopySign => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f32.copysign(b.num.f32) });
            }
            F64Eq => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 == b.num.f64 } as i32);
            }
            F64Neq => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 != b.num.f64 } as i32);
            }
            F64Lt => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 < b.num.f64 } as i32);
            }
            F64Gt => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 > b.num.f64 } as i32);
            }
            F64Le => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 <= b.num.f64 } as i32);
            }
            F64Ge => {
                let b = stack.pop_value();
                let a = stack.pop_value();
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
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 + b.num.f64 });
            }
            F64Sub => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 - b.num.f64 });
            }
            F64Mul => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 * b.num.f64 });
            }
            F64Div => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64 / b.num.f64 });
            }
            F64Min => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64.min(b.num.f64) });
            }
            F64Max => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64.max(b.num.f64) });
            }
            F64CopySign => {
                let b = stack.pop_value();
                let a = stack.pop_value();
                stack.push_value(unsafe { a.num.f64.copysign(b.num.f64) });
            }
            TableGet { table_idx } => {
                let i = stack.pop_value();
                let frame = stack.current_frame();
                let tableaddr = store.instances.resolve(frame.winst_id).addr_of(*table_idx);
                let table = store.tables.resolve(tableaddr);
                stack.push_value(table.elems[unsafe { i.num.i32 } as usize]);
            }
            TableSet { table_idx } => {
                let val = stack.pop_value();
                let i = stack.pop_value();
                let frame = stack.current_frame();
                let tableaddr = store.instances.resolve(frame.winst_id).addr_of(*table_idx);
                let table = store.tables.resolve_mut(tableaddr);
                table.elems[unsafe { i.num.i32 } as usize] = unsafe { val.ref_ };
            }
            TableSize { table_idx } => {
                let frame = stack.current_frame();
                let tableaddr = store.instances.resolve(frame.winst_id).addr_of(*table_idx);
                let table = store.tables.resolve(tableaddr);
                stack.push_value(table.elems.len() as i32);
            }
            TableGrow { table_idx } => {
                let n = unsafe { stack.pop_value().num.i32 } as usize;
                let val = unsafe { stack.pop_value().ref_ };
                let frame = stack.current_frame();
                let tableaddr = store.instances.resolve(frame.winst_id).addr_of(*table_idx);
                let table = store.tables.resolve_mut(tableaddr);
                let sz = table.elems.len();
                if let Some(max) = table.type_.limits.max {
                    if sz + n > (max as usize) {
                        stack.push_value(-1i32);
                        ip = unsafe { ip.add(1) };
                        continue;
                    }
                }
                table.elems.reserve(n as usize);
                for _ in 0..n {
                    table.elems.push(val);
                }
                stack.push_value(sz as i32);
            }
            TableFill { table_idx } => {
                let frame = stack.current_frame();
                let tableaddr = store.instances.resolve(frame.winst_id).addr_of(*table_idx);
                let table = store.tables.resolve_mut(tableaddr);
                let n = unsafe { stack.pop_value().num.i32 } as usize;
                let val = unsafe { stack.pop_value().ref_ };
                let i = unsafe { stack.pop_value().num.i32 } as usize;
                if i + n > table.elems.len() {
                    return Err(WasmTrap {});
                }
                for idx in i..(i + n) {
                    table.elems[idx] = val;
                }
            }
            TableCopy { dst, src } => {
                let frame = stack.current_frame();
                let winst = store.instances.resolve(frame.winst_id);
                let tableaddr_dst = winst.addr_of(*dst);
                let tableaddr_src = winst.addr_of(*src);
                let (table_dst, table_src) =
                    store.tables.resolve_multi_mut(tableaddr_dst, tableaddr_src);
                let n = unsafe { stack.pop_value().num.i32 } as usize;
                let s = unsafe { stack.pop_value().num.i32 } as usize;
                let d = unsafe { stack.pop_value().num.i32 } as usize;
                if s + n > table_src.elems.len() {
                    return Err(WasmTrap {});
                }
                if d + n > table_dst.elems.len() {
                    return Err(WasmTrap {});
                }
                if n > 0 {
                    (&mut table_dst.elems[d..(d + n)])
                        .copy_from_slice(&table_src.elems[s..(s + n)]);
                }
            }
            TableInit {
                table_idx,
                elem_idx,
            } => {
                let n = unsafe { stack.pop_value().num.i32 } as usize;
                let s = unsafe { stack.pop_value().num.i32 } as usize;
                let d = unsafe { stack.pop_value().num.i32 } as usize;
                let frame = stack.current_frame();
                let winst = store.instances.resolve(frame.winst_id);
                let table = store.tables.resolve_mut(winst.addr_of(*table_idx));
                let elem = store.elems.resolve(winst.addr_of(*elem_idx));
                (&mut table.elems[d..(d + n)]).copy_from_slice(&elem.elem[s..(s + n)]);
            }
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
            GlobalGet { global_idx } => {
                let frame = stack.current_frame();
                let globaladdr = store.instances.resolve(frame.winst_id).addr_of(*global_idx);
                stack.push_value(store.globals.resolve(globaladdr).val);
            }
            GlobalSet { global_idx } => {
                let val = stack.pop_value();
                let frame = stack.current_frame();
                let globaladdr = store.instances.resolve(frame.winst_id).addr_of(*global_idx);
                store.globals.resolve_mut(globaladdr).val = val;
            }
            LocalGet { local_idx } => {
                let frame = stack.current_frame();
                let val = frame.locals[local_idx.0 as usize];
                stack.push_value(val);
            }
            LocalSet { local_idx } => {
                let val = stack.pop_value();
                let frame = stack.current_frame_mut();
                frame.locals[local_idx.0 as usize] = val;
            }
            LocalTee { local_idx } => {
                let val = stack.pop_value();
                let frame = stack.current_frame_mut();
                frame.locals[local_idx.0 as usize] = val;
                stack.push_value(val);
            }
            Unreachable => return Err(WasmTrap {}),
            Nop => {}
            Block { block_type: _, imm } => {
                stack.push_label(WasmLabel {
                    instr: unsafe { ip.add(imm.0 as usize) },
                });
            }
            Loop {
                block_type: _,
                imm: _,
            } => {
                stack.push_label(WasmLabel { instr: ip });
            }
            If { block_type: _, imm } => {
                let val = stack.pop_value();
                if (unsafe { val.num.i32 } != 0) {
                    stack.push_label(WasmLabel {
                        instr: unsafe { ip.add(imm.end_off.0 as usize) },
                    });
                } else {
                    if let Some(else_off) = imm.else_off {
                        ip = unsafe { ip.add(else_off.0 as usize) };
                        stack.push_label(WasmLabel {
                            instr: unsafe { ip.add(imm.end_off.0 as usize) },
                        });
                    } else {
                        ip = unsafe { ip.add(imm.end_off.0 as usize) };
                        continue;
                    }
                }
            }
            Else => {
                let label = stack.pop_label(WasmLabelIdx(0));
                ip = label.instr;
            }
            ExprEnd => match stack.pop_control() {
                Some(ControlStackEntry::Frame(_frame)) => {
                    if let Some(ControlStackEntry::Label(label)) = stack.pop_control() {
                        ip = label.instr;
                        continue;
                    } else {
                        break;
                    }
                }
                Some(ControlStackEntry::Label(_label)) => {}
                None => break,
            },
            Break { label_idx } => {
                let label = stack.pop_label(*label_idx);
                ip = label.instr;
                continue;
            }
            BreakIf { label_idx } => {
                let val = stack.pop_value();
                if (unsafe { val.num.i32 } != 0) {
                    let label = stack.pop_label(*label_idx);
                    ip = label.instr;
                    continue;
                }
            }
            BreakTable {
                labels,
                default_label,
            } => {
                todo!();
            }
            Return => {
                stack.pop_frame();
                if let Some(ControlStackEntry::Label(label)) = stack.pop_control() {
                    ip = label.instr;
                    continue;
                } else {
                    break;
                }
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
                        func: funcimpl,
                    } => {
                        let mut locals = args;
                        for local_type in &funcimpl.locals {
                            locals.push(WasmValue::default_of_type(local_type));
                        }
                        stack.push_label(WasmLabel {
                            instr: unsafe { ip.add(1) },
                        });
                        stack.push_frame(WasmFrame {
                            locals: locals.into_boxed_slice(),
                            winst_id,
                        });
                        ip = &funcimpl.body[0];
                        continue;
                    }
                }
            }
            CallIndirect {
                table_idx,
                type_idx,
            } => {
                todo!();
            }
            RefNull { ref_type: _ } => {
                stack.push_value(WasmRefValue::NULL);
            }
            RefIsNull => {
                let addr = unsafe { stack.pop_value().ref_.func };
                stack.push_value(addr.is_null() as i32);
            }
            RefFunc { func_idx } => {
                let frame = stack.current_frame();
                let funcaddr = store.instances.resolve(frame.winst_id).addr_of(*func_idx);
                stack.push_value(WasmRefValue { func: funcaddr });
            }
            Drop => {
                stack.pop_value();
            }
            Select { value_types: _ } => {
                let c = unsafe { stack.pop_value().num.i32 };
                let val1 = stack.pop_value();
                let val2 = stack.pop_value();
                if c != 0 {
                    stack.push_value(val1);
                } else {
                    stack.push_value(val2);
                }
            }
            instr @ _ => panic!("instr unimplemented: {:?}", instr),
        }
        ip = unsafe { ip.add(1) };
    }
    Ok(())
}
