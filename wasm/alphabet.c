
#define WASM_PAGE_SIZE 65536

extern void puts(char *str);
extern unsigned long __builtin_wasm_memory_size(int);
extern unsigned long __builtin_wasm_memory_grow(int, unsigned long);
extern char __heap_base[];

void* sbce_malloc(int bytes) {
    int current_size = __builtin_wasm_memory_size(0) * WASM_PAGE_SIZE;
    if (current_size < bytes) {
        __builtin_wasm_memory_grow(0, (bytes - current_size) / WASM_PAGE_SIZE);
    }
    return &__heap_base;
}

void alphabet(int reps) {
    char *str = sbce_malloc((26 * reps) + 1);
    int idx = 0;
    while (reps--) {
        for (char c = 'a'; c <= 'z'; c++) {
            str[idx++] = c;
        }
    }
    str[idx] = 0;
    puts(str);
}
