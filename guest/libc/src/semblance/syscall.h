
#pragma once

#include "../inttypes.h"

#define WASM_IMPORT(module, name) __attribute__((import_module(module), import_name(name)))

// ----- proc -------- //

WASM_IMPORT("semblance", "exit")
extern void semblance_syscall_exit(int) __attribute__((noreturn));

WASM_IMPORT("semblance", "panic")
extern void semblance_syscall_panic(const char *msg) __attribute__((noreturn));

// ----- io -------- //

WASM_IMPORT("semblance", "fopen")
extern int32_t semblance_syscall_fopen(const char *path, const char *mode);

WASM_IMPORT("semblance", "fwrite")
extern int32_t semblance_syscall_fwrite(int fd, void *data, size_t len);

WASM_IMPORT("semblance", "ftell")
extern int64_t semblance_syscall_ftell(int fd);

WASM_IMPORT("semblance", "fflush")
extern int32_t semblance_syscall_fflush(int fd);

WASM_IMPORT("semblance", "fread")
extern int32_t semblance_syscall_fread(int fd, void *dst, size_t size);

WASM_IMPORT("semblance", "fclose")
extern int32_t semblance_syscall_fclose(int fd);

// ----- fs -------- //

WASM_IMPORT("semblance", "remove")
extern int32_t semblance_syscall_remove(const char *path);

WASM_IMPORT("semblance", "rename")
extern int32_t semblance_syscall_rename(const char *path1, const char *path2);

// ----- util -------- //

WASM_IMPORT("semblance", "parse_f64")
extern double semblance_syscall_parse_f64(const char *str);

WASM_IMPORT("semblance", "parse_i32")
extern int32_t semblance_syscall_parse_i32(const char *str);

// ------ gfx --------- //

WASM_IMPORT("semblance", "init_window")
extern void semblance_syscall_init_window(const char *title, int32_t width, int32_t height);

WASM_IMPORT("semblance", "set_window_title")
extern void semblance_syscall_set_window_title(const char *title);
