#!/usr/bin/env bash

SEMBLANCE=target/debug/semblance
WAST_PATH="${1:?}"
WORK_DIR="${2:?}"
TEMP_WASM_DIR="$(mktemp -d)"
SUCCESSES=0
FAILURES=0
SKIPPED=0
CMD="null"

function cleanup() {
    rm -r "$TEMP_WASM_DIR"
}

trap cleanup EXIT

function load_wast() {
    wasm-tools json-from-wast "$WAST_PATH" --wasm-dir "$TEMP_WASM_DIR" \
        | jq -c '.commands[]'
}

function pass() {
    ((SUCCESSES++))
    echo "$CMD" >>"$WORK_DIR/cmd_log.passed.jsonl"
}

function fail() {
    ((FAILURES++))
    echo "$CMD" >>"$WORK_DIR/cmd_log.failed.jsonl"
}

function skip() {
    ((SKIPPED++))
    echo "$CMD" >>"$WORK_DIR/cmd_log.skipped.jsonl"
}

while read -r CMD; do
    echo "$CMD" >>"$WORK_DIR/cmd_log.jsonl"
    CMD_TYPE="$(echo "$CMD" | jq -r '.type')"
    case "$CMD_TYPE" in
        module)
            FILENAME="$(echo "$CMD" | jq -r '.filename')"
            WASM_FILE="$TEMP_WASM_DIR/$FILENAME"
        ;;
        assert_return)
            ACTION="$(echo "$CMD" | jq -r '.action')"
            INVOKE_FN="$(echo "$ACTION" | jq -r '.field')"
            INVOKE_ARGS=$(echo "$ACTION" | jq -r '[.args[].value] | join(" ")')
            RETURN_ARGS=$(echo "$CMD" | jq -r '[.expected[].value] | join(" ")')
            if $SEMBLANCE "$WASM_FILE" --invoke "$INVOKE_FN" $INVOKE_ARGS --assert-return $RETURN_ARGS; then
                pass
            else
                fail
            fi
        ;;
        assert_malformed)
            if [[ "$(echo "$CMD" | jq -r '.module_type')" == "text" ]]; then
                echo "skipping assert_malformed for .wat -- $(echo "$CMD" | jq -r '.text')"
                skip
            else
                MALFORMED_WASM_FILE="$TEMP_WASM_DIR/$(echo "$CMD" | jq -r '.filename')"
                if $SEMBLANCE "$MALFORMED_WASM_FILE" --assert-malformed; then
                    pass
                else
                    fail
                fi
            fi
        ;;
        assert_invalid)
            INVALID_WASM_FILE="$TEMP_WASM_DIR/$(echo "$CMD" | jq -r '.filename')"
            if $SEMBLANCE "$INVALID_WASM_FILE" --assert-invalid; then
                pass
            else
                fail
            fi
        ;;
        assert_*)
            skip
        ;;
    esac
done < <(load_wast) 1>&2

TOTAL_TESTS="$((SUCCESSES + FAILURES + SKIPPED))"

echo "$SUCCESSES" >"$WORK_DIR/success_count"
echo "$FAILURES" >"$WORK_DIR/failure_count"
echo "$SKIPPED" >"$WORK_DIR/skipped_count"
echo "$TOTAL_TESTS" >"$WORK_DIR/total_count"

# ------------------ #

echo "PASSED: $SUCCESSES"
echo "FAILED: $FAILURES"
echo "SKIPPED: $SKIPPED"
echo "TOTAL: $TOTAL_TESTS"

exit $FAILURES
