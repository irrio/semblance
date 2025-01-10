#!/usr/bin/env bash

SRC=${1:?}
OUT="$(basename $SRC .c).wasm"

# Credit: https://surma.dev/things/c-to-webassembly/

clang \
    --target=wasm32 \
    -nostdlib \
    -Wl,--no-entry \
    -Wl,--export-all \
    -o "$OUT" \
    "$SRC"
