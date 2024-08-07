
typedef struct {
    char *path;
} CliArgs;

int cli_parse(CliArgs *args, int argc, char *argv[]);
