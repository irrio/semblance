
extern void init(int argc, char **argv);
extern void tick();

#define WASM_EXPORT(name) __attribute__((export_name(name)))

WASM_EXPORT("_start")
void _start() {
    int argc = 1;
    char *argv[1] = { "/doomgeneric.wasm" };
    return init(argc, argv);
}

WASM_EXPORT("_tick")
void _tick() {
    tick();
}
