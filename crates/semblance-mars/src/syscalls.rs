use std::{f64, sync::LazyLock};

use semblance::{
    inst::{WasmNumValue, WasmStore, WasmValue, table::WasmInstanceAddr},
    link::WasmLinker,
    module::{WasmFuncType, WasmNumType, WasmResultType, WasmValueType},
};

use crate::{guest_gfx, guest_io, syscalls::util::guest_resolve_cstr};

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

static SYSCALL_INIT_WINDOW_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // char *title
        WasmValueType::Num(WasmNumType::I32), // int32_t width
        WasmValueType::Num(WasmNumType::I32), // int32_t height
    ])),
    output_type: WasmResultType(Box::new([])),
});

fn syscall_init_window(
    store: &mut WasmStore,
    winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let title = unsafe { args[0].num.i32 };
    let width = unsafe { args[1].num.i32 } as u32;
    let height = unsafe { args[2].num.i32 } as u32;
    let title = util::guest_resolve_cstr(store, winst_id, title);
    eprintln!("[guest] init_window(\"{}\", {}, {})", title, width, height);
    guest_gfx::create_window(title, width, height);
    Box::new([])
}

static SYSCALL_SET_WINDOW_TITLE_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // char *title
    ])),
    output_type: WasmResultType(Box::new([])),
});

fn syscall_set_window_title(
    store: &mut WasmStore,
    winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let title = unsafe { args[0].num.i32 };
    let title = guest_resolve_cstr(store, winst_id, title);
    eprintln!("[guest] set_window_title({})", title);
    guest_gfx::use_window_mut(|w| w.set_title(title)).expect("failed to set window title");
    Box::new([])
}

static SYSCALL_PARSE_I32_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // char *str
    ])),
    output_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::I32)])),
});

fn syscall_parse_i32(
    store: &mut WasmStore,
    winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let str = unsafe { args[0].num.i32 };
    let str = guest_resolve_cstr(store, winst_id, str);
    eprintln!("[guest] parse_i32({})", str);
    Box::new([WasmValue {
        num: WasmNumValue {
            i32: str.parse().unwrap_or(-1),
        },
    }])
}

static SYSCALL_PARSE_F64_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // char *str
    ])),
    output_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::F64)])),
});

fn syscall_parse_f64(
    store: &mut WasmStore,
    winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let str = unsafe { args[0].num.i32 };
    let str = guest_resolve_cstr(store, winst_id, str);
    eprintln!("[guest] parse_f64({})", str);
    Box::new([WasmValue {
        num: WasmNumValue {
            f64: str.parse().unwrap_or(f64::NAN),
        },
    }])
}

static SYSCALL_FOPEN_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // char *path
        WasmValueType::Num(WasmNumType::I32), // char *mode
    ])),
    output_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::I32)])),
});

fn syscall_fopen(
    store: &mut WasmStore,
    winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let path = unsafe { args[0].num.i32 };
    let path = guest_resolve_cstr(store, winst_id, path);
    let mode = unsafe { args[1].num.i32 };
    let mode = guest_resolve_cstr(store, winst_id, mode);
    eprintln!("[guest] fopen({}, {})", path, mode);
    let fd = guest_io::fopen(path, mode);
    Box::new([WasmValue {
        num: WasmNumValue { i32: fd },
    }])
}

static SYSCALL_PANIC_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // char *msg
    ])),
    output_type: WasmResultType(Box::new([])),
});

fn syscall_panic(
    store: &mut WasmStore,
    winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let msg = unsafe { args[0].num.i32 };
    let msg = guest_resolve_cstr(store, winst_id, msg);
    eprintln!("[guest] panic({})", msg);
    panic!("guest panicked: {}", msg);
}

pub fn add_to_linker(linker: &mut WasmLinker) {
    linker.add_host_module(
        "semblance".to_string(),
        &[
            ("exit", &SYSCALL_EXIT_TYPE, &syscall_exit),
            (
                "init_window",
                &SYSCALL_INIT_WINDOW_TYPE,
                &syscall_init_window,
            ),
            (
                "set_window_title",
                &SYSCALL_SET_WINDOW_TITLE_TYPE,
                &syscall_set_window_title,
            ),
            ("parse_i32", &SYSCALL_PARSE_I32_TYPE, &syscall_parse_i32),
            ("parse_f64", &SYSCALL_PARSE_F64_TYPE, &syscall_parse_f64),
            ("fopen", &SYSCALL_FOPEN_TYPE, &syscall_fopen),
            ("panic", &SYSCALL_PANIC_TYPE, &syscall_panic),
        ],
    );
}

mod util {

    use super::*;
    use semblance::module::WasmMemIdx;
    use std::ffi::CStr;

    pub fn guest_resolve_cstr(store: &WasmStore, winst_id: WasmInstanceAddr, addr: i32) -> &str {
        let addr = addr as u32 as usize;
        let winst = store.instances.resolve(winst_id);
        let mem = store.mems.resolve(winst.addr_of(WasmMemIdx::ZERO));
        if addr > mem.data.len() {
            panic!("cstr addr {} out of bounds", addr);
        }
        let cstr = CStr::from_bytes_until_nul(&mem.data[addr..]).expect("invalid cstr from guest");
        cstr.to_str().expect("invalid utf8 in guest str")
    }
}
