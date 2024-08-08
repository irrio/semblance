
#include <sys/types.h>

typedef struct {
    int fd;
    off_t size;
    void *data;
} WasmModule;

typedef struct {
    int wmod_err;
    int cause;
} WmodErr;

WmodErr wmod_read(WasmModule *wmod, char *path);

int wmod_failed(WmodErr);
char *wmod_str_error(WmodErr err);
char *wmod_str_error_cause(WmodErr err);
