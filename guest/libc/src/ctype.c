
#include "ctype.h"

int isspace(int c) {
    switch (c) {
        case ' ':
        case '\t':
        case '\n':
        case '\v':
        case '\f':
        case '\r':
            return 1;
        default:
            return 0;
    }
}

int toupper(int c) {
    if (c >= 'a' && c <= 'z')
        return c - ('a' - 'A');
    return c;
}
