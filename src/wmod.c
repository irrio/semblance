
#include "wmod.h"
#include "vec.h"
#include <stdio.h>

void wmod_init(WasmModule *wmod) {
    vec_init(&wmod->types);
    vec_init(&wmod->funcs);
    vec_init(&wmod->tables);
    vec_init(&wmod->mems);
    vec_init(&wmod->globals);
    vec_init(&wmod->elems);
    vec_init(&wmod->datas);
    vec_init(&wmod->start);
    vec_init(&wmod->imports);
    vec_init(&wmod->exports);
    vec_init(&wmod->customs);
    wmod->meta.version = 0;
}

void wmod_dump(WasmModule *wmod) {
    printf("version: %d\n", wmod->meta.version);
    printf("types: %zu\n", wmod->types.len);
}
