
CC=clang
CFLAGS=-std=c99 -Wall -Wextra

target/semblance: src/main.c target/cli.o target/wmod.o target/wbin.o target/wrun.o target/vec.o target/
	$(CC) $(CFLAGS) src/main.c target/cli.o target/wmod.o target/wbin.o target/wrun.o target/vec.o -o target/semblance

target/cli.o: src/cli.c src/cli.h target/
	$(CC) $(CFLAGS) -c src/cli.c -o target/cli.o

target/wmod.o: src/wmod.c src/wmod.h target/
	$(CC) $(CFLAGS) -c src/wmod.c -o target/wmod.o

target/wbin.o: src/wbin.c src/wbin.h src/wmod.h target/
	$(CC) $(CFLAGS) -c src/wbin.c -o target/wbin.o

target/wrun.o: src/wrun.c src/wrun.h src/wmod.h target/
	$(CC) $(CFLAGS) -c src/wrun.c -o target/wrun.o

target/vec.o: src/vec.c src/vec.h target/
	$(CC) $(CFLAGS) -c src/vec.c -o target/vec.o

target/:
	mkdir -p target

target/hello.wasm: wasm/hello.c target/
	./scripts/c2wasm.sh wasm/hello.c

run: target/semblance target/hello.wasm
	./target/semblance target/hello.wasm --invoke hello

.PHONY: clean run

clean:
	rm -rf target
