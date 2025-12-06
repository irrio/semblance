#!/usr/bin/env zsh

zmodload zsh/datetime
setopt KSH_ARRAYS

now_millis() {
    local NOW_SECS="${epochtime[0]}"
    local NOW_NANOS="${epochtime[1]}"
    echo $(( NOW_SECS * 1000 + (NOW_NANOS / 1000000) ))
}

SEMBLANCE=target/release/semblance
NUM_WORKERS=8
START_TIME=$(now_millis)
WORK_DIR=$(mktemp -d -p ./teststate/runs "$(printf "%x" "$START_TIME")-XXXX")
TEST_RUN=$(basename "$WORK_DIR")
DB="./teststate/analytics.duckdb"

echo "running" >"$WORK_DIR/status"
mkdir "$WORK_DIR/worker"

for ((i=0; i<NUM_WORKERS; i++)); do
    mkdir "$WORK_DIR/worker/$i"
    mkfifo "$WORK_DIR/worker/$i/queue"
    (
        PASSED=0
        FAILED=0
        exec {log_fd}>>"$WORK_DIR/worker/$i/worker.log"
        exec {csv_fd}>>"$WORK_DIR/worker/$i/test_case_executions.csv"

        log() {
            echo "[test.sh] $@" >&$log_fd
        }

        write_csv() {
            echo "$@" >&$csv_fd
        }

        while read -r SUITE_NAME TEST_CASE_ID ARGS; do
            TEST_CASE_START_TIME=$(now_millis)
            log "$TEST_CASE_ID: $SEMBLANCE $ARGS"
            "$SEMBLANCE" ${(z)ARGS} >&$log_fd 2>&1
            EXIT_CODE="$?"
            TEST_CASE_END_TIME=$(now_millis)
            if [[ $EXIT_CODE -eq 0 ]]; then
                ((PASSED++))
            else
                ((FAILED++))
            fi
            log "EXITED: $EXIT_CODE"
            write_csv "$TEST_RUN,$SUITE_NAME,$TEST_CASE_ID,$EXIT_CODE,$TEST_CASE_START_TIME,$TEST_CASE_END_TIME"
        done <"$WORK_DIR/worker/$i/queue"

        echo "$PASSED" >"$WORK_DIR/worker/$i/passed_count"
        echo "$FAILED" >"$WORK_DIR/worker/$i/failed_count"
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

declare -i TOTAL_TESTS
TOTAL_TESTS=$(cat teststate/suites/*/commands.txt | wc -l)
TESTS_QUEUED=0

render_percent() {
    local -i PERCENT;
    PERCENT="${1:?}"
    printf "\r["
    for ((i=0;i<=25;i++)); do
        if (( $i * 4 <= $PERCENT )); then
            printf "="
        else
            printf " "
        fi
    done
    printf "]"
}

report_progress() {
    if (( TESTS_QUEUED == TOTAL_TESTS || (TESTS_QUEUED % 50) == 0 )); then
        local PERCENT=$(bc -e "($TESTS_QUEUED / $TOTAL_TESTS) * 100" --scale 2)
        local PROGRESS_BAR=$(render_percent "$PERCENT")
        printf "\r%s" "$PROGRESS_BAR"
    fi
}

round_robin() {
    while read -r LINE; do
        idx=$(( TESTS_QUEUED % NUM_WORKERS ))
        echo "$LINE" >&${QUEUE_FDS[$idx]}
        ((TESTS_QUEUED++))
        report_progress
    done
}

cat teststate/suites/*/commands.txt | round_robin

for ((i=0; i<NUM_WORKERS; i++)); do
    fd="${QUEUE_FDS[$i]}"
    exec {fd}>&-
done

wait
END_TIME=$(now_millis)

PASSED="$(awk '{ sum += $1 } END { print sum }' $WORK_DIR/worker/*/passed_count)"
FAILED="$(awk '{ sum += $1 } END { print sum }' $WORK_DIR/worker/*/failed_count)"

declare -i PANICS
PANICS=$(grep -o panicked $WORK_DIR/worker/*/worker.log | wc -l)

echo "$PANICS" >"$WORK_DIR/panic_count"
echo "$PASSED" >"$WORK_DIR/passed_count"
echo "$FAILED" >"$WORK_DIR/failed_count"

if [[ "$FAILED" -eq 0 ]]; then
    echo "passed" >"$WORK_DIR/status"
    PASSED_BOOL=true
else
    echo "failed" >"$WORK_DIR/status"
    PASSED_BOOL=false
fi

if [ -z "$(git status --porcelain)" ]; then
    GIT_DIRTY=false
else
    GIT_DIRTY=true
fi

duckdb "$DB" <<<"insert into test_run(id, exe, git_commit_sha, git_dirty, started_at, finished_at, passed)
    VALUES ('$TEST_RUN', '$SEMBLANCE', '$(git rev-parse HEAD)', '$GIT_DIRTY', make_timestamp_ms($START_TIME), make_timestamp_ms($END_TIME), $PASSED_BOOL);
    insert into test_case_execution(test_run_id,suite_name,test_case,exit_code,started_at,finished_at)
    select
        column0 as test_run_id,
        column1 as suite_name,
        column2 as test_case,
        column3 as exit_code,
        make_timestamp_ms(column4) as started_at,
        make_timestamp_ms(column5) as finished_at
    from read_csv('./teststate/runs/$TEST_RUN/worker/*/test_case_executions.csv')"

TEST_DURATION=$(( (END_TIME - START_TIME) / 1000 ))
ELAPSED_MINS=$(( TEST_DURATION / 60 ))
SECS_REMAINING=$(( TEST_DURATION % 60 ))
echo "\r---------------------------------------------"
echo "Completed in ${ELAPSED_MINS}m ${SECS_REMAINING}s -- $PANICS panics detected"
echo "---------------------------------------------"
printf "%-10s %-10s\n" PASSED "$PASSED"
printf "%-10s %-10s\n" FAILED "$FAILED"
echo "---------------------------------------------"
./scripts/trendline.sh
echo "$WORK_DIR"
