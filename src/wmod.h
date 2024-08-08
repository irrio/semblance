
#include <sys/types.h>

typedef struct {
    char magic_bytes[4];
    u_int32_t version;
    u_int8_t sections[];
} WasmHeader;

typedef struct {
    off_t size;
    WasmHeader *data;
} WasmModule;

typedef struct {
    int wmod_err;
    int cause;
} WmodErr;

WmodErr wmod_read(WasmModule *wmod, char *path);

int wmod_failed(WmodErr);
char *wmod_str_error(WmodErr err);
char *wmod_str_error_cause(WmodErr err);
