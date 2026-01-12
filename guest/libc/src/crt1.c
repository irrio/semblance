
#include "internal/stdio.h"
#include "semblance/syscall.h"

extern void init(int argc, char **argv);
extern void tick();

#define WASM_EXPORT(name) __attribute__((export_name(name)))

WASM_EXPORT("_start")
void _start() {
    int stdio_err = __stdio_init();
    if (stdio_err) semblance_syscall_panic("failed to initialize stdio");

    int argc = 1;
    char *argv[1] = { "/doomgeneric.wasm" };
    return init(argc, argv);
}

WASM_EXPORT("_tick")
void _tick() {
    tick();
}
