#!/usr/bin/env bash

SEMBLANCE=target/semblance
WAST_PATH="${1:?}"
TEMP_WASM_DIR="$(mktemp -d)"
SUCCESSES=0
FAILURES=0

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
            if $SEMBLANCE "$WASM_FILE" --invoke "$INVOKE_FN" $INVOKE_ARGS; then
                echo "assert_return passed"
                ((SUCCESSES++))
            else
                ((FAILURES++))
            fi
        ;;
        assert_*)
            echo "unhandled $CMD_TYPE"
            ((FAILURES++))
        ;;
    esac
done < <(load_wast)

echo "$SUCCESSES/$((SUCCESSES + FAILURES)) tests passed"

exit $FAILURES
