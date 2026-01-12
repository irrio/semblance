use std::{path::PathBuf, rc::Rc};

use sdl2::event::Event;
use semblance::{inst::WasmInvokeOptions, link::WasmLinker, module::WasmModule};

mod guest_gfx;
mod guest_io;
mod syscalls;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module_path = std::env::args().nth(1).expect("missing module path");
    let module_path = PathBuf::from(module_path);
    let mut linker = WasmLinker::new();
    syscalls::add_to_linker(&mut linker);
    let wmod = WasmModule::read(&module_path).expect("unable to load module");
    let (mut store, externvals) = linker.link(&wmod).expect("unable to resolve imports");
    let winst_id = store
        .instantiate(Rc::new(wmod), &externvals)
        .expect("failed to instantiate");
    let winst = store.instances.resolve(winst_id);
    let initfunc = winst
        .resolve_export_fn_by_name("_start")
        .expect("no _start func exported");
    let tickfunc = winst
        .resolve_export_fn_by_name("_tick")
        .expect("no _tick func exported");
    store
        .invoke(initfunc, Box::new([]), WasmInvokeOptions::default())
        .expect("guest trapped during init");

    eprintln!("Guest initialized!");

    let mut event_pump =
        guest_gfx::use_sdl_context(|ctx| ctx.event_pump().expect("failed to get event pump"));
    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. } => {
                    break 'running;
                }
                _ => {
                    store
                        .invoke(tickfunc, Box::new([]), WasmInvokeOptions::default())
                        .expect("guest trapped during _tick");
                }
            }
        }
    }

    Ok(())
}
