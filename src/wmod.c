
#include "wmod.h"
#include <fcntl.h>
#include <errno.h>
#include <stdio.h>
#include <string.h>
#include <sys/mman.h>
#include <sys/types.h>
#include <sys/stat.h>
#include <unistd.h>

const int WMOD_ERR_IO = 1;

WmodErr wmod_mk_err(int wmod_err, int cause) {
    WmodErr out;
    out.wmod_err = wmod_err;
    out.cause = cause;
    return out;
}

WmodErr wmod_err_ok() {
    WmodErr out;
    out.wmod_err = 0;
    out.cause = 0;
    return out;
}

WmodErr wmod_read(WasmModule *wmod, char *path) {
    int fd = open(path, O_RDONLY);
    if (fd < 0) return wmod_mk_err(WMOD_ERR_IO, errno);
    wmod->fd = fd;

    struct stat stats;
    int stat_err = fstat(fd, &stats);
    if (stat_err == -1) return wmod_mk_err(WMOD_ERR_IO, errno);
    wmod->size = stats.st_size;

    void* data = mmap(NULL, stats.st_size, PROT_READ, MAP_PRIVATE, fd, 0);
    if ((size_t) data == -1) return wmod_mk_err(WMOD_ERR_IO, errno);

    wmod->data = data;

    return wmod_err_ok();
}

int wmod_failed(WmodErr err) {
    return err.wmod_err != 0;
}

char *wmod_str_error(WmodErr err) {
    switch (err.wmod_err) {
        case WMOD_ERR_IO:
            return "unable to open file";
        default:
            return "unknown error";
    }
}

char *wmod_str_error_cause(WmodErr err) {
    switch (err.wmod_err) {
        case WMOD_ERR_IO:
            return strerror(err.cause);
        case 0:
            return "";
        default:
            return "unknown cause";
    }
}
