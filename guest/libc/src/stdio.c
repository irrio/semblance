
#include "stdio.h"
#include "internal/stdio.h"
#include "semblance/syscall.h"
#include "stdlib.h"
#include "string.h"

typedef enum stream_kind_t {
    stream_kind_fd,
    stream_kind_buf
} stream_kind_t;

typedef struct buf_stream_t {
    void *start;
    void *end;
} buf_stream_t;

struct FILE {
    stream_kind_t kind;
    union {
        buf_stream_t buf_state;
        int fd;
    } data;
};

static void FILE_init_buf(FILE *f, char *buf, size_t len) {
    if (f == NULL) return;
    f->kind = stream_kind_buf;
    f->data.buf_state.start = buf;
    f->data.buf_state.end = buf + len;
}

static void FILE_init_fd(FILE *f, int fd) {
    if (f == NULL) return;
    f->kind = stream_kind_fd;
    f->data.fd = fd;
}

static size_t bufwrite(buf_stream_t *buf_state, const void *data, size_t len) {
    size_t buf_size = buf_state->end - buf_state->start;
    if (len > buf_size) return 0;
    __builtin_memcpy(buf_state->start, data, len);
    buf_state->start += len;
    return len;
}

static size_t bufread(buf_stream_t *buf_state, void *dst, size_t len) {
    size_t buf_size = buf_state->end - buf_state->start;
    if (len > buf_size) return 0;
    __builtin_memcpy(dst, buf_state->start, len);
    buf_state->start += len;
    return len;
}

static size_t fwrite_str(FILE *f, char *str) {
    size_t len = strlen(str);
    return fwrite(str, sizeof(char), len, f);
}

static size_t fwrite_char(FILE *f, char c) {
    return fwrite(&c, sizeof(char), 1, f);
}

static size_t num_digits_of_base(uint64_t n, uint32_t base) {
    size_t digits = 1;
    while (n /= base) digits++;
    return (size_t) digits;
}

const char *DIGITS_UPPER = "0123456789ABCDEF";
const char *DIGITS_LOWER = "0123456789abcdef";

static size_t fwrite_uint(FILE *f, uint64_t i, uint32_t base, const char *digitstr) {
    size_t written = 0;
    size_t digits = num_digits_of_base(i, base);
    for (; digits > 0; digits--) {
        size_t place = digits > 1 ? (base * (digits - 1)) : 1;
        size_t digit = (i / place) % base;
        written += fwrite(&digitstr[digit], sizeof(char), 1, f);
    }
    return written;
}

static size_t fwrite_int(FILE *f, int64_t i, uint32_t base, const char *digitstr) {
    size_t written = 0;
    uint64_t rest;
    if (i < 0) {
        written += fwrite("-", sizeof(char), 1, f);
        rest = (uint64_t)(0 - i);
    } else {
        rest = i;
    }
    written += fwrite_uint(f, rest, base, digitstr);
    return written;
}

static size_t fwrite_ptr(FILE *f, void *ptr) {
    size_t written = 0;
    written += fwrite("0x", sizeof(char), 2, f);
    written += fwrite_uint(f, (uint64_t)ptr, 16, DIGITS_UPPER);
    return written;
}

static size_t fwrite_hex(FILE *f, uint32_t i, const char *digitstr) {
    size_t written = 0;
    written += fwrite("0x", sizeof(char), 2, f);
    written += fwrite_uint(f, (uint64_t)i, 16, digitstr);
    return written;
}

FILE* stderr = NULL;
FILE* stdout = NULL;

int __stdio_init() {
    stderr = fopen("/dev/stderr", "w");
    if (stderr == NULL) return 1;
    stdout = fopen("/dev/stdout", "w");
    if (stderr == NULL) return 2;
    return 0;
}

int printf(const char *format, ...) {
    va_list args;
    va_start(args, format);
    int result = vfprintf(stdout, format, args);
    va_end(args);
    return result;
}

int fprintf(FILE *f, const char *format, ...) {
    va_list args;
    va_start(args, format);
    int result = vfprintf(f, format, args);
    va_end(args);
    return result;
}

int vfprintf(FILE *stream, const char *format, va_list arg) {
    int written = 0;
    char *format_end = strchr(format, '\0');
    while (format < format_end) {
        char *pat = strchr(format, '%');
        if (pat == NULL) {
            written += fwrite(format, sizeof(char), format_end - format, stream);
            break;
        } else {
            written += fwrite(format, sizeof(char), pat - format, stream);
            switch (pat[1]) {
                case '\0':
                case '%': {
                    written += fwrite_char(stream, '%');
                    break;
                }
                case 's': {
                    char *str = va_arg(arg, char*);
                    written += fwrite_str(stream, str);
                    break;
                }
                case 'p': {
                    void *ptr = va_arg(arg, void*);
                    written += fwrite_ptr(stream, ptr);
                    break;
                }
                case 'x': {
                    uint32_t i = va_arg(arg, uint32_t);
                    written += fwrite_hex(stream, i, DIGITS_LOWER);
                    break;
                }
                case 'X': {
                    uint32_t i = va_arg(arg, uint32_t);
                    written += fwrite_hex(stream, i, DIGITS_UPPER);
                    break;
                }
                case 'd':
                case 'i': {
                    int32_t i = va_arg(arg, int32_t);
                    written += fwrite_int(stream, i, 10, DIGITS_UPPER);
                    break;
                }
                default: {
                    va_arg(arg, int);
                    written += fwrite(pat, sizeof(char), 2, stream);
                    break;
                }
            }
            format = &pat[2];
        }
    }
    return written;
}

int snprintf(char *str, size_t size, const char *format, ...) {
    va_list args;
    va_start(args, format);
    int result = vsnprintf(str, size, format, args);
    va_end(args);
    return result;
}

int vsnprintf(char *buffer, size_t size, const char *format, va_list argptr) {
    FILE f;
    FILE_init_buf(&f, buffer, size - 1);
    int written = vfprintf(&f, format, argptr);
    if (written > 0) {
        buffer[written] = '\0';
    }
    return written;
}

int puts(const char *str) {
    size_t len = strlen(str);
    size_t written = fwrite(str, sizeof(char), len, stdout);
    if (written != len) return EOF;
    if (putchar('\n') == EOF) return EOF;
    return len + 1;
}

int putchar(int c) {
    uint8_t data = c;
    size_t written = fwrite(&data, sizeof(uint8_t), 1, stdout);
    if (written != 1) return EOF;
    return (int)data;
}

FILE *fopen(const char *path, const char *mode) {
    int fd = semblance_syscall_fopen(path, mode);
    if (fd < 0) return NULL;
    FILE *f = malloc(sizeof(FILE));
    if (f == NULL) return NULL;
    FILE_init_fd(f, fd);
    return f;
}

size_t fread(void *ptr, size_t size, size_t nmemb, FILE *stream) {
    size_t read = 0;
    if (stream == NULL) return 0;
    switch (stream->kind) {
        case stream_kind_fd:
            read = semblance_syscall_fread(stream->data.fd, ptr, size * nmemb);
            break;
        case stream_kind_buf:
            read = bufread(&stream->data.buf_state, ptr, size * nmemb);
            break;
    }
    return read / size;
}

int fseek(FILE *stream, long int offset, int whence) {
    return 0;
}

size_t fwrite(const void *ptr, size_t size, size_t nmemb, FILE *stream) {
    size_t written = 0;
    if (stream == NULL) return 0;
    switch (stream->kind) {
        case stream_kind_fd: {
            written = semblance_syscall_fwrite(stream->data.fd, ptr, size * nmemb);
            break;
        }
        case stream_kind_buf: {
            written = bufwrite(&stream->data.buf_state, ptr, size * nmemb);
            break;
        }
    }
    return written / size;
}

int fclose(FILE *f) {
    if (f == NULL) return -1;
    if (f->kind == stream_kind_fd) {
        return semblance_syscall_fclose(f->data.fd);
    }
    return 0;
}

int fflush(FILE *f) {
    if (f == NULL) return -1;
    if (f->kind == stream_kind_fd) {
        return semblance_syscall_fflush(f->data.fd);
    }
    return 0;
}

long ftell(FILE *f) {
    if (f == NULL) return -1;
    if (f->kind == stream_kind_fd) {
        return semblance_syscall_ftell(f->data.fd);
    }
    return -1;
}

int remove(const char *path) {
    return -1;
}

int rename(const char *src, const char *dst) {
    return -1;
}

int sscanf(const char *str, const char *format, ...) {
    return 0;
}
