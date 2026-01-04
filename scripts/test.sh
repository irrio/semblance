#!/usr/bin/env zsh

zmodload zsh/datetime
setopt KSH_ARRAYS

now_millis() {
    local NOW_SECS="${epochtime[0]}"
    local NOW_NANOS="${epochtime[1]}"
    echo $(( NOW_SECS * 1000 + (NOW_NANOS / 1000000) ))
}

DB=teststate/analytics.duckdb

createdb() {
    if [[ ! -f "$DB" ]]; then
        duckdb "$DB" <<END_SQL

CREATE TABLE test_run (
    id              VARCHAR PRIMARY KEY,
    exe             VARCHAR NOT NULL,
    git_commit_sha  VARCHAR NOT NULL,
    git_dirty       BOOLEAN NOT NULL,
    started_at      TIMESTAMP NOT NULL,
    finished_at     TIMESTAMP NOT NULL
);

CREATE TABLE wast_execution (
    id                  UUID PRIMARY KEY DEFAULT uuidv7(),
    test_run_id         VARCHAR NOT NULL REFERENCES test_run(id),
    wast_path           VARCHAR NOT NULL,
    exit_code           INTEGER NOT NULL,
    passed_directives   INTEGER NOT NULL,
    total_directives    INTEGER NOT NULL,
    started_at          TIMESTAMP NOT NULL,
    finished_at         TIMESTAMP NOT NULL
);

END_SQL

    fi
}

mkdir -p teststate/runs
cargo build --package semblance-wast || exit 1
createdb

SEMBLANCE_HARNESS=target/debug/semblance-wast
NUM_WORKERS=8
START_TIME=$(now_millis)
WORK_DIR=$(mktemp -d -p ./teststate/runs "$(printf "%x" "$START_TIME")-XXXX")
TEST_RUN=$(basename "$WORK_DIR")

mkdir "$WORK_DIR/worker"

for ((i=0; i<NUM_WORKERS; i++)); do
    mkdir "$WORK_DIR/worker/$i"
    mkfifo "$WORK_DIR/worker/$i/queue"
    (
        exec {log_fd}>>"$WORK_DIR/worker/$i/worker.log"
        exec {csv_fd}>>"$WORK_DIR/worker/$i/results.csv"

        log() {
            echo "[test.sh] $@" >&$log_fd
        }

        write_csv() {
            echo "$@" >&$csv_fd
        }

        while read -r WAST_SUITE; do
            log "Running $WAST_SUITE"
            SUITE_START_TIME=$(now_millis)
            "$SEMBLANCE_HARNESS" "$WAST_SUITE" >&$log_fd 2>&1
            EXIT_CODE="$?"
            SUITE_END_TIME=$(now_millis)
            SUITE_DURATION=$(( SUITE_END_TIME - SUITE_START_TIME ))
            log "EXITED with code $EXIT_CODE after ${SUITE_DURATION}ms"
            DIRECTIVE_TEXT="$(tail -r "$WORK_DIR/worker/$i/worker.log" | grep -m 1 -o '\[\d\+/\d\+\]')"
            DIRECTIVE_NUMS="${"${DIRECTIVE_TEXT#'['}"%']'}";
            DIRECTIVE_PARTS=("${(@s:/:)DIRECTIVE_NUMS}")
            TOTAL_DIRECTIVES="${DIRECTIVE_PARTS[1]}"
            if [[ "$EXIT_CODE" == 0 ]]; then
                PASSED_DIRECTIVES="$TOTAL_DIRECTIVES"
            else
                PASSED_DIRECTIVES="${DIRECTIVE_PARTS[0]}"
            fi
            write_csv "$TEST_RUN,$WAST_SUITE,$EXIT_CODE,$PASSED_DIRECTIVES,$TOTAL_DIRECTIVES,$SUITE_START_TIME,$SUITE_END_TIME"
        done <"$WORK_DIR/worker/$i/queue"

        rm "$WORK_DIR/worker/$i/queue"
        exec {log_fd}>&-
        exec {csv_fd}>&-
    ) &
done

declare -a QUEUE_FDS

for ((i=0; i<NUM_WORKERS; i++)); do
    exec {fd}>"$WORK_DIR/worker/$i/queue"
    QUEUE_FDS+=("$fd")
done

TESTS_QUEUED=0

round_robin() {
    while read -r LINE; do
        idx=$(( TESTS_QUEUED % NUM_WORKERS ))
        echo "$LINE" >&${QUEUE_FDS[$idx]}
        ((TESTS_QUEUED++))
    done
}

list_suites() {
    for SUITE in spec/test/core/*.wast; do
        if [[ "$SUITE" != *"names.wast" ]]; then
            echo "$SUITE"
        fi
    done
}

list_suites | round_robin

for ((i=0; i<NUM_WORKERS; i++)); do
    fd="${QUEUE_FDS[$i]}"
    exec {fd}>&-
done

wait
END_TIME=$(now_millis)

if [ -z "$(git status --porcelain)" ]; then
    GIT_DIRTY=false
else
    GIT_DIRTY=true
fi

duckdb "$DB" <<<"insert into test_run(id, exe, git_commit_sha, git_dirty, started_at, finished_at)
    VALUES ('$TEST_RUN', '$SEMBLANCE_HARNESS', '$(git rev-parse HEAD)', '$GIT_DIRTY', make_timestamp_ms($START_TIME), make_timestamp_ms($END_TIME));
    insert into wast_execution(test_run_id,wast_path,exit_code,passed_directives,total_directives,started_at,finished_at)
    select
        column0 as test_run_id,
        column1 as wast_path,
        column2 as exit_code,
        column3 as passed_directives,
        column4 as total_directives,
        make_timestamp_ms(column5) as started_at,
        make_timestamp_ms(column6) as finished_at
    from read_csv('./teststate/runs/$TEST_RUN/worker/*/results.csv')"

TEST_DURATION=$(( (END_TIME - START_TIME) ))
GRADE=$(duckdb "$DB" -ascii -noheader -c "select PRINTF('%.2f', (sum(passed_directives) / sum(total_directives)) * 100) as percentage from wast_execution where test_run_id='$TEST_RUN';")
echo "---------------------------------------------"
echo "Completed in ${TEST_DURATION}ms with $GRADE% passing"
echo "---------------------------------------------"
echo "$WORK_DIR"
