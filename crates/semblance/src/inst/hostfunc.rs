use crate::inst::{WasmInstanceAddr, WasmStack, WasmStore, WasmValue};

pub type WasmHostFunc = &'static dyn WasmCallable;

pub struct WasmHostCallContext<'s> {
    pub store: &'s mut WasmStore,
    pub stack: &'s mut WasmStack,
    pub inst: WasmInstanceAddr,
}

pub trait WasmCallable {
    fn call(&self, args: &[WasmValue], ctx: &mut WasmHostCallContext);
}

impl<F> WasmCallable for F
where
    F: Fn(&mut WasmStore, WasmInstanceAddr, &[WasmValue]) -> Box<[WasmValue]>,
{
    fn call(&self, args: &[WasmValue], ctx: &mut WasmHostCallContext) {
        let ret = self(&mut ctx.store, ctx.inst, args);
        for val in ret {
            ctx.stack.push_value(val);
        }
    }
}
