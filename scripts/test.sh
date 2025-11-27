#!/usr/bin/env zsh

START_TIME=$(date +%s)
WORK_DIR=$(mktemp -d)

echo "$WORK_DIR"

cleanup() {
    rm -rf $WORK_DIR;
}

trap cleanup EXIT;

declare -A TEST_PIDS;
declare -A COMPLETED_TESTS;

mkfifo "$WORK_DIR/completions";

for WAST in ./testsuite/*.wast; do
    (
        TEST_DIR="$WORK_DIR/$(basename $WAST)"
        mkdir "$TEST_DIR"
        sleep 3
        ./scripts/run-wast.sh "$WAST" "$TEST_DIR" >"$TEST_DIR/run-wast.log" 2>&1
        RETRIES=5
        while (( RETRIES > 0 )); do
            if [[ ! -e "$WORK_DIR/completions" ]]; then
                exit
            fi
            echo $WAST >$WORK_DIR/completions;
            ((RETRIES--));
            sleep 1
        done
    ) &
    TEST_PIDS[$WAST]="$!";
done

echo "Started ${#TEST_PIDS} test suites..."

while (( ${#TEST_PIDS} > 0 )); do
    if (( ${#TEST_PIDS} < 5 )); then
        echo ">> Waiting on (${(@k)TEST_PIDS[@]})"
    fi
    read WAST <$WORK_DIR/completions
    if [[ ${+TEST_PIDS[$WAST]} -eq 1 ]]; then
        unset "TEST_PIDS[$WAST]"
        COMPLETED_TESTS[$WAST]=1
        echo "$WAST completed ${#TEST_PIDS} remaining"
    fi
done

rm $WORK_DIR/completions;
END_TIME=$(date +%s)

PASSED_SUITES=0
FAILED_SUITES=0
SUCCESSES=0
FAILURES=0
SKIPPED=0

for WAST in "${(@k)COMPLETED_TESTS}"; do
    TEST_DIR="$WORK_DIR/$(basename $WAST)";
    if [[ $(cat $TEST_DIR/failure_count) -eq 0 ]]; then
        PASSED_SUITES=$(( PASSED_SUITES + 1 ))
    else
        FAILED_SUITES=$(( FAILED_SUITES + 1 ))
    fi
    SUCCESSES=$(( SUCCESSES + "$(cat $TEST_DIR/success_count)" ))
    FAILURES=$(( FAILURES + "$(cat $TEST_DIR/failure_count)" ))
    SKIPPED=$(( SKIPPED + "$(cat $TEST_DIR/skipped_count)" ))
done

echo "------------------------------"
printf "%-20s %-4s\n" "PASSED SUITES" "$PASSED_SUITES"
printf "%-20s %-4s\n" "FAILED SUITES" "$FAILED_SUITES"
echo "------------------------------"
printf "%-20s %-4s\n" "SUCCESSES" "$SUCCESSES"
printf "%-20s %-4s\n" "FAILURES" "$FAILURES"
printf "%-20s %-4s\n" "SKIPPED" "$SKIPPED"
echo "------------------------------"

ELAPSED_SECONDS=$(( END_TIME - START_TIME ))
ELAPSED_MINUTES=$(( ELAPSED_SECONDS / 60 ))
REM_SECONDS=$(( ELAPSED_SECONDS % 60 ))
echo "Test run took ${ELAPSED_MINUTES}m ${REM_SECONDS}s"

echo "$WORK_DIR"
