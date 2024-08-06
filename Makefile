
build:
	clang src/main.c -o target/semblance

run: build
	./target/semblance

clean:
	rm -rf target/*
