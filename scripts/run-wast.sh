#!/usr/bin/env bash

SEMBLANCE=target/debug/semblance
WAST_PATH="${1:?}"
TEMP_WASM_DIR="$(mktemp -d)"
SUCCESSES=0
FAILURES=0
SKIPPED=0

function cleanup() {
    rm -r "$TEMP_WASM_DIR"
}

trap cleanup EXIT

function load_wast() {
    wasm-tools json-from-wast "$WAST_PATH" --wasm-dir "$TEMP_WASM_DIR" \
        | jq -c '.commands[]'
}

while read -r CMD; do
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
                ((SUCCESSES++))
            else
                ((FAILURES++))
            fi
        ;;
        assert_malformed)
            if [[ "$(echo "$CMD" | jq -r '.module_type')" == "text" ]]; then
                echo "skipping assert_malformed for .wat -- $(echo "$CMD" | jq -r '.text')"
                ((SKIPPED++))
            else
                MALFORMED_WASM_FILE="$TEMP_WASM_DIR/$(echo "$CMD" | jq -r '.filename')"
                if $SEMBLANCE "$MALFORMED_WASM_FILE" --assert-malformed; then
                    ((FAILURES++))
                else
                    ((SUCCESSES++))
                fi
            fi
        ;;
        assert_invalid)
            INVALID_WASM_FILE="$TEMP_WASM_DIR/$(echo "$CMD" | jq -r '.filename')"
            if $SEMBLANCE "$INVALID_WASM_FILE" --assert-invalid; then
                ((FAILURES++))
            else
                ((SUCCESSES++))
            fi
        ;;
        assert_*)
            echo "unhandled $CMD_TYPE"
            ((SKIPPED++))
        ;;
    esac
done < <(load_wast) 1>&2

TOTAL_TESTS="$((SUCCESSES + FAILURES + SKIPPED))"
echo "PASSED: $SUCCESSES"
echo "FAILED: $FAILURES"
echo "SKIPPED: $SKIPPED"
echo "TOTAL: $TOTAL_TESTS"

exit $FAILURES
