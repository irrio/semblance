
# Semblance

Semblance is an in-place WebAssembly interpreter written in C.
The implementation is a rather direct translation of the [WebAssembly Core Specification](https://webassembly.github.io/spec/core/)
into C code. Everything is much easier to grok if you are familiar with the spec.

I wanted to understand the WebAssembly core specification in detail, and this project is
the result of that. As such, this is not production quality software. There are many
places where I've skipped input validation in an effort to move forward quickly.
One should be careful to only evaluate trusted WebAssembly modules with this interpreter.

I'm still a far way off from implementing [every opcode](https://webassembly.github.io/spec/core/syntax/instructions.html).
I doubt it will ever get that far. But I've made enough progress that simple `Hello, World`
programs will run without issue.

## Build Instructions

Compile the interpreter by running `make`. The executable will be found
at `target/semblance`.

```bash
make
```

## Usage Instructions

```text
semblance <MODULE.wasm>

Options:
    -h, --help              Print this help text
    -I, --invoke <NAME>     Invoke the function exported as $NAME
```

Currently, the runtime provides an implementation of `void puts(char *str);` that
can be used to write to standard output.

```C
extern void puts(char *str);

void hello() {
    puts("Hello, World!");
}
```

Compile this C code to WebAssembly with Clang:

```bash
clang \
    --target=wasm32 \
    -O3 \
    # Don't include the standard library
    -nostdlib \
    # Don't include an entry point
    -Wl,--no-entry \
    # Export all symbols in the resulting module
    -Wl,--export-all \
    # Allow any undefined symbols (like puts) to be
    # found in the environment, provided by the runtime
    -Wl,--allow-undefined \
    -o hello.wasm \
    hello.c
```

Run `hello.wasm`:

```bash
./target/semblance hello.wasm --invoke hello
```

This should output:

```text
Hello, World!
Ok []
```

## Contributing

Since this project is primarily for my personal learning purposes,
I am currently not accepting any pull requests. However, I am
pretty inexperienced writing C and would happily accept a
code review from someone willing to provide feedback.
