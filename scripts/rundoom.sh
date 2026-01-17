#!/usr/bin/env zsh

cd guest/libc

make clean && make

cd ../doomgeneric

make clean && make

cd ../..

cargo run --release --package semblance-mars -- ./guest/doomgeneric/target/doomgeneric.wasm
