
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

static size_t num_digits_of_base(uint64_t n, uint32_t base) {
    size_t digits = 1;
    while (n /= base) digits++;
    return digits;
}

const char *DIGITS_UPPER = "0123456789ABCDEF";
const char *DIGITS_LOWER = "0123456789abcdef";

typedef enum printf_specifier_kind_t {
    printf_specifier_kind_none,
    printf_specifier_kind_int,
    printf_specifier_kind_uint,
    printf_specifier_kind_str,
    printf_specifier_kind_ptr,
    printf_specifier_kind_hex,
    printf_specifier_kind_float,
} printf_specifier_kind_t;

#define PRINTF_SPECIFIER_FLAG_PAD_ZERO (1 << 0)
#define PRINTF_SPECIFIER_FLAG_HEX_UPPER (1 << 1)

typedef struct printf_specifier_t {
    printf_specifier_kind_t kind;
    uint8_t flags;
    int width;
    int precision;
} printf_specifier_t;

static const char *parse_printf_specifier_flags(const char* format, uint8_t *flags) {
    switch (format[0]) {
        case '0':
            *flags |= PRINTF_SPECIFIER_FLAG_PAD_ZERO;
            return &format[1];
    }
    return format;
}

static const int is_digit(char c) {
    return c >= '0' && c <= '9';
}

static const int digit_val(char c) {
    return (int)c - (int)'0';
}

static size_t powi(size_t base, size_t exp) {
    size_t raised = 1;
    while (exp--) {
        raised *= base;
    }
    return raised;
}

static const int parse_int(const char *c, int numdigits) {
    int val = 0;
    for (int i = 0; i < numdigits; i++) {
        val = (val * 10) + digit_val(c[i]);
    }
    return val;
}

static const char *take_int(const char* format, int *out) {
    size_t idx = 0;
    while (is_digit(format[idx])) idx++;
    if (idx > 0) {
        *out = parse_int(format, idx);
        return &format[idx];
    }
    return format;
}

static const char *take_char(const char* format, char c) {
    return format[0] == c ? &format[1] : format;
}

static const char *parse_printf_specifier_precision(const char* format, int *precision) {
    if (format[0] == '.') {
        return take_int(&format[1], precision);
    }
    return format;
}

static const char *parse_printf_specifier_kind(const char *format, printf_specifier_kind_t *kind, uint8_t *flags) {
    switch (format[0]) {
        case '\0':
        case '%':
            *kind = printf_specifier_kind_none;
            break;
        case 'i':
        case 'd':
            *kind = printf_specifier_kind_int;
            break;
        case 'u':
            *kind = printf_specifier_kind_uint;
            break;
        case 'f':
            *kind = printf_specifier_kind_float;
            break;
        case 's':
            *kind = printf_specifier_kind_str;
            break;
        case 'p':
            *kind = printf_specifier_kind_ptr;
            break;
        case 'X':
            *flags |= PRINTF_SPECIFIER_FLAG_HEX_UPPER;
        case 'x':
            *kind = printf_specifier_kind_hex;
            break;
        default:
            semblance_syscall_panic("unknown printf specifier kind");
            break;
    }
    return &format[1];
}

static const char *parse_printf_specifier(const char *format, printf_specifier_t *out) {
    __builtin_memset(out, 0, sizeof(printf_specifier_t));
    format = take_char(format, '%');
    format = parse_printf_specifier_flags(format, &out->flags);
    format = take_int(format, &out->width);
    format = parse_printf_specifier_precision(format, &out->precision);
    format = parse_printf_specifier_kind(format, &out->kind, &out->flags);
    return format;
}

static size_t fwrite_str(FILE *f, char *str, printf_specifier_t *specifier) {
    size_t len = strlen(str);
    if (specifier->precision > 0) {
        len = specifier->precision < len ? specifier->precision : len;
    }
    return fwrite(str, sizeof(char), len, f);
}

static size_t fwrite_char(FILE *f, char c) {
    return fwrite(&c, sizeof(char), 1, f);
}

static size_t fwrite_uint(FILE *f, uint64_t i, uint32_t base, printf_specifier_t *specifier) {
    const char *digitstr = specifier->flags & PRINTF_SPECIFIER_FLAG_HEX_UPPER ? DIGITS_UPPER : DIGITS_LOWER;
    size_t written = 0;
    size_t actual_digits = num_digits_of_base(i, base);
    size_t min_digits = specifier->precision > 0
        ? specifier->precision
        : specifier->width > 0
            ? specifier->width
            : 0;
    size_t digits = min_digits > actual_digits ? min_digits : actual_digits;
    for (; digits > 0; digits--) {
        size_t place = powi(base, digits - 1);
        size_t digit = (i / place) % base;
        written += fwrite(&digitstr[digit], sizeof(char), 1, f);
    }
    return written;
}

static size_t fwrite_int(FILE *f, int64_t i, uint32_t base, printf_specifier_t *specifier) {
    size_t written = 0;
    uint64_t rest;
    if (i < 0) {
        written += fwrite("-", sizeof(char), 1, f);
        rest = (uint64_t)(0 - i);
    } else {
        rest = i;
    }
    written += fwrite_uint(f, rest, base, specifier);
    return written;
}

static size_t fwrite_ptr(FILE *f, void *ptr, printf_specifier_t *specifier) {
    size_t written = 0;
    written += fwrite("0x", sizeof(char), 2, f);
    written += fwrite_uint(f, (uint64_t)ptr, 16, specifier);
    return written;
}

static size_t fwrite_hex(FILE *f, uint32_t i, printf_specifier_t *specifier) {
    size_t written = 0;
    written += fwrite_uint(f, (uint64_t)i, 16, specifier);
    return written;
}

static size_t fwrite_float(FILE *f, double num, printf_specifier_t *specifier) {
    size_t written = 0;
    int whole = (int)num;
    int fractional = (int)((num - (double)whole) * (specifier->precision * 10));
    int width = specifier->width;
    int precision = specifier->precision;
    specifier->precision = 0;
    written += fwrite_int(f, whole, 10, specifier);
    specifier->width = 0;
    specifier->precision = precision;
    written += fwrite_uint(f, fractional, 10, specifier);
    specifier->width = width;
    return written;
}

static size_t fwrite_printf_specifier(FILE *f, printf_specifier_t *specifier, va_list *vargs) {
    va_list args = *vargs;
    size_t written = 0;
    switch (specifier->kind) {
        case printf_specifier_kind_none:
            written += fwrite("%", sizeof(char), 1, f);
            break;
        case printf_specifier_kind_int: {
            written += fwrite_int(f, va_arg(args, int), 10, specifier);
            break;
        }
        case printf_specifier_kind_uint:
            written += fwrite_uint(f, va_arg(args, uint32_t), 10, specifier);
            break;
        case printf_specifier_kind_hex:
            written += fwrite_hex(f, va_arg(args, int), specifier);
            break;
        case printf_specifier_kind_ptr:
            written += fwrite_ptr(f, va_arg(args, void*), specifier);
            break;
        case printf_specifier_kind_str:
            written += fwrite_str(f, va_arg(args, char*), specifier);
            break;
        case printf_specifier_kind_float:
            written += fwrite_float(f, va_arg(args, double), specifier);
            break;
    }
    *vargs = args;
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

int vfprintf(FILE *stream, const char *format, va_list args) {
    int written = 0;
    char *format_end = strchr(format, '\0');
    while (format < format_end) {
        char *pat = strchr(format, '%');
        if (pat == NULL) {
            written += fwrite(format, sizeof(char), format_end - format, stream);
            break;
        } else {
            written += fwrite(format, sizeof(char), pat - format, stream);
            printf_specifier_t specifier;
            format = parse_printf_specifier(pat, &specifier);
            written += fwrite_printf_specifier(stream, &specifier, &args);
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
    if (stream == NULL) return 1;
    if (stream->kind == stream_kind_fd) {
        return semblance_syscall_fseek(stream->data.fd, offset, whence);
    }
    return 1;
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
