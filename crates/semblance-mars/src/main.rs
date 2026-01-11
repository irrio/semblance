use std::{path::PathBuf, rc::Rc, sync::LazyLock};

use sdl2::event::Event;
use semblance::{
    inst::{WasmInvokeOptions, WasmStore, WasmValue, table::WasmInstanceAddr},
    link::WasmLinker,
    module::{WasmFuncType, WasmModule, WasmNumType, WasmResultType, WasmValueType},
};

static SYSCALL_EXIT_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::I32)])),
    output_type: WasmResultType(Box::new([])),
});

fn syscall_exit(
    _store: &mut WasmStore,
    _winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let status = unsafe { args[0].num.i32 };
    eprintln!("[guest] exit({})", status);
    std::process::exit(status);
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let module_path = std::env::args().nth(1).expect("missing module path");
    let module_path = PathBuf::from(module_path);
    let mut linker = WasmLinker::new();
    linker.add_host_module(
        "semblance".to_string(),
        &[("exit", &SYSCALL_EXIT_TYPE, &syscall_exit)],
    );
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

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Semblance Mars", 800, 480)
        .position_centered()
        .build()?;

    eprintln!("Opened SDL2 Window: {}", window.title());

    let mut event_pump = sdl_context.event_pump()?;
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
