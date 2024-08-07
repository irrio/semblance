
target/semblance: target/cli.o
	clang src/main.c target/cli.o -o target/semblance

target/cli.o: src/cli.c src/cli.h
	clang -c src/cli.c -o target/cli.o

run: target/semblance
	./target/semblance wasm/two.wasm

clean:
	rm -rf target/*
