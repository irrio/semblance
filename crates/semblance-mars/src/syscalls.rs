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
    let fd = guest_io::fopen(path, mode);
    Box::new([WasmValue {
        num: WasmNumValue { i32: fd },
    }])
}

static SYSCALL_FREAD_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // fd
        WasmValueType::Num(WasmNumType::I32), // void *data
        WasmValueType::Num(WasmNumType::I32), // len
    ])),
    output_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::I32)])),
});

fn syscall_fread(
    store: &mut WasmStore,
    winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let fd = unsafe { args[0].num.i32 };
    let data = unsafe { args[1].num.i32 };
    let len = unsafe { args[2].num.i32 };
    let slice = util::guest_resolve_slice_mut(store, winst_id, data, len);
    let read = guest_io::fread(fd, slice);
    Box::new([WasmValue {
        num: WasmNumValue { i32: read },
    }])
}

static SYSCALL_FCLOSE_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // fd
    ])),
    output_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::I32)])),
});

fn syscall_fclose(
    _store: &mut WasmStore,
    _winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let fd = unsafe { args[0].num.i32 };
    let res = guest_io::fclose(fd);
    Box::new([WasmValue {
        num: WasmNumValue { i32: res },
    }])
}

static SYSCALL_FTELL_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // fd
    ])),
    output_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::I64)])),
});

fn syscall_ftell(
    _store: &mut WasmStore,
    _winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let fd = unsafe { args[0].num.i32 };
    let res = guest_io::ftell(fd);
    Box::new([WasmValue {
        num: WasmNumValue { i64: res },
    }])
}

static SYSCALL_FSEEK_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // fd
        WasmValueType::Num(WasmNumType::I64), // offset
        WasmValueType::Num(WasmNumType::I32), // whence
    ])),
    output_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::I32)])),
});

fn syscall_fseek(
    _store: &mut WasmStore,
    _winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let fd = unsafe { args[0].num.i32 };
    let offset = unsafe { args[1].num.i64 };
    let whence = unsafe { args[2].num.i32 };
    let res = guest_io::fseek(fd, offset, whence);
    Box::new([WasmValue {
        num: WasmNumValue { i32: res },
    }])
}

static SYSCALL_FFLUSH_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // fd
    ])),
    output_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::I32)])),
});

fn syscall_fflush(
    _store: &mut WasmStore,
    _winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let fd = unsafe { args[0].num.i32 };
    let res = guest_io::fflush(fd);
    Box::new([WasmValue {
        num: WasmNumValue { i32: res },
    }])
}

static SYSCALL_FWRITE_TYPE: LazyLock<WasmFuncType> = LazyLock::new(|| WasmFuncType {
    input_type: WasmResultType(Box::new([
        WasmValueType::Num(WasmNumType::I32), // fd
        WasmValueType::Num(WasmNumType::I32), // void *data
        WasmValueType::Num(WasmNumType::I32), // len
    ])),
    output_type: WasmResultType(Box::new([WasmValueType::Num(WasmNumType::I32)])),
});

fn syscall_fwrite(
    store: &mut WasmStore,
    winst_id: WasmInstanceAddr,
    args: &[WasmValue],
) -> Box<[WasmValue]> {
    let fd = unsafe { args[0].num.i32 };
    let data = unsafe { args[1].num.i32 };
    let len = unsafe { args[2].num.i32 };
    let slice = util::guest_resolve_slice(store, winst_id, data, len);
    let written = guest_io::fwrite(fd, slice);
    Box::new([WasmValue {
        num: WasmNumValue { i32: written },
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
            ("fclose", &SYSCALL_FCLOSE_TYPE, &syscall_fclose),
            ("fread", &SYSCALL_FREAD_TYPE, &syscall_fread),
            ("fwrite", &SYSCALL_FWRITE_TYPE, &syscall_fwrite),
            ("ftell", &SYSCALL_FTELL_TYPE, &syscall_ftell),
            ("fseek", &SYSCALL_FSEEK_TYPE, &syscall_fseek),
            ("fflush", &SYSCALL_FFLUSH_TYPE, &syscall_fflush),
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

    pub fn guest_resolve_slice(
        store: &WasmStore,
        winst_id: WasmInstanceAddr,
        addr: i32,
        len: i32,
    ) -> &[u8] {
        let addr = addr as u32 as usize;
        let len = len as u32 as usize;
        let winst = store.instances.resolve(winst_id);
        let mem = store.mems.resolve(winst.addr_of(WasmMemIdx::ZERO));
        mem.data
            .get(addr..(addr + len))
            .expect("slice out of range")
    }

    pub fn guest_resolve_slice_mut(
        store: &mut WasmStore,
        winst_id: WasmInstanceAddr,
        addr: i32,
        len: i32,
    ) -> &mut [u8] {
        let addr = addr as u32 as usize;
        let len = len as u32 as usize;
        let winst = store.instances.resolve(winst_id);
        let mem = store.mems.resolve_mut(winst.addr_of(WasmMemIdx::ZERO));
        mem.data
            .get_mut(addr..(addr + len))
            .expect("slice out of range")
    }
}
