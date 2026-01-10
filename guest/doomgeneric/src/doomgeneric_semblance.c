
#include "doomgeneric.h"
#include <stdint.h>

void DG_Init() {
    // ...
}

void DG_SetWindowTitle(const char *title) {
    //
}

void DG_DrawFrame() {
    // ...
}

void DG_SleepMs(uint32_t ms) {
    // ...
}

uint32_t DG_GetTicksMs() {
    return 0;
}

int DG_GetKey(int* pressed, unsigned char* doomKey) {
    return 0;
}

int main(int argc, char** argv) {
    doomgeneric_Create(argc, argv);
    while (1) {
        doomgeneric_Tick();
    }
}

__attribute__((export_name("run")))
void run() {
    char* args[1] = { "/doomgeneric.wasm" };
    main(1, args);
}
