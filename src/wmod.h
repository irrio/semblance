
typedef struct {

} WasmModule;

int wmod_read(WasmModule *wmod, char *path);

char *wmod_str_error(int err);
