
typedef struct {
    char *path;
} CliArgs;

int cli_parse(CliArgs *args, int argc, char *argv[]);

char *cli_str_error(int err);

char *cli_usage_str();
