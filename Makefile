
target/semblance: src/main.c target/cli.o target/wmod.o target/wbin.o target/wrun.o target/vec.o
	clang src/main.c target/cli.o target/wmod.o target/wbin.o target/wrun.o target/vec.o -o target/semblance

target/cli.o: src/cli.c src/cli.h
	clang -c src/cli.c -o target/cli.o

target/wmod.o: src/wmod.c src/wmod.h
	clang -c src/wmod.c -o target/wmod.o

target/wbin.o: src/wbin.c src/wbin.h src/wmod.h
	clang -c src/wbin.c -o target/wbin.o

target/wrun.o: src/wrun.c src/wrun.h src/wmod.h
	clang -c src/wrun.c -o target/wrun.o

target/vec.o: src/vec.c src/vec.h
	clang -c src/vec.c -o target/vec.o

run: target/semblance
	./target/semblance wasm/hello.wasm

clean:
	rm -rf target/*
