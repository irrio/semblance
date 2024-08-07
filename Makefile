
target/semblance: src/main.c target/cli.o target/wmod.o
	clang src/main.c target/cli.o target/wmod.o -o target/semblance

target/cli.o: src/cli.c src/cli.h
	clang -c src/cli.c -o target/cli.o

target/wmod.o: src/wmod.c src/wmod.h
	clang -c src/wmod.c -o target/wmod.o

run: target/semblance
	./target/semblance wasm/two.wasm

clean:
	rm -rf target/*
