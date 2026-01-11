
extern int main(int argc, char **argv);

int _start() {
    int argc = 1;
    char *argv[1] = { "/doomgeneric.wasm" };
    return main(argc, argv);
}
