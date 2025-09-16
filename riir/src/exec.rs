use crate::{
    inst::{WasmNumValue, WasmRefValue, WasmStack, WasmStore, WasmTrap},
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
            I32Add => {
                let a = stack.pop_value();
                let b = stack.pop_value();
                stack.push_value(unsafe { a.num.i32 + b.num.i32 });
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
                (&mut mem.data[d..n]).copy_from_slice(&data_bytes[s..n]);
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
            Unreachable => return Err(WasmTrap {}),
            ExprEnd => break,
            instr @ _ => panic!("instr unimplemented: {:?}", instr),
        }
        ic += 1;
    }
    Ok(())
}
