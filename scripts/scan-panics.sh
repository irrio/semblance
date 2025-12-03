
TEST_RUN_ID="$1"

if [[ -z "$TEST_RUN_ID" ]]; then
    TEST_RUN_ID="$(ls teststate/runs | sort -r | head -n 1)"
fi

WORK_DIR="teststate/runs/$TEST_RUN_ID"

grep -h -A 1 panicked $WORK_DIR/worker/*/worker.log | less --long-prompt
