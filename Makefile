
target/semblance: src/main.c target/cli.o target/wmod.o target/wbin.o target/leb128.o
	clang src/main.c target/cli.o target/wmod.o target/wbin.o target/leb128.o -o target/semblance

target/cli.o: src/cli.c src/cli.h
	clang -c src/cli.c -o target/cli.o

target/wmod.o: src/wmod.c src/wmod.h
	clang -c src/wmod.c -o target/wmod.o

target/wbin.o: src/wbin.c src/wbin.h src/wmod.h src/leb128.h
	clang -c src/wbin.c -o target/wbin.o

target/leb128.o: src/leb128.c src/leb128.h
	clang -c src/leb128.c -o target/leb128.o

run: target/semblance
	./target/semblance wasm/two.wasm

clean:
	rm -rf target/*
