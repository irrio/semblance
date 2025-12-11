
__attribute__((import_module("constants")))
__attribute__((import_name("two")))
extern int two();

__attribute__((import_module("ops")))
__attribute__((import_name("add")))
extern int add(int a, int b);

int add_two(int a) {
    return add(a, two());
}
