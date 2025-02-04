
target/semblance: src/main.c target/cli.o target/wmod.o target/wbin.o target/wrun.o target/vec.o target/
	clang src/main.c target/cli.o target/wmod.o target/wbin.o target/wrun.o target/vec.o -o target/semblance

target/cli.o: src/cli.c src/cli.h target/
	clang -c src/cli.c -o target/cli.o

target/wmod.o: src/wmod.c src/wmod.h target/
	clang -c src/wmod.c -o target/wmod.o

target/wbin.o: src/wbin.c src/wbin.h src/wmod.h target/
	clang -c src/wbin.c -o target/wbin.o

target/wrun.o: src/wrun.c src/wrun.h src/wmod.h target/
	clang -c src/wrun.c -o target/wrun.o

target/vec.o: src/vec.c src/vec.h target/
	clang -c src/vec.c -o target/vec.o

target/:
	mkdir -p target

run: target/semblance
	./target/semblance wasm/hello.wasm --invoke hello

clean:
	rm -rf target/*
