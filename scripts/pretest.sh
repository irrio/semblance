#!/usr/bin/env zsh

TEST_STATE_DIR="./teststate"
DB="$TEST_STATE_DIR/analytics.duckdb"

mkdir -p "$TEST_STATE_DIR/suites"
mkdir -p "$TEST_STATE_DIR/runs"

create_analytics_db() {
    duckdb "$DB" <<END_SQL

CREATE TABLE IF NOT EXISTS test_run (
    id              VARCHAR PRIMARY KEY,
    exe             VARCHAR NOT NULL,
    git_commit_sha  VARCHAR NOT NULL,
    git_dirty       BOOLEAN NOT NULL,
    started_at      TIMESTAMP NOT NULL,
    finished_at     TIMESTAMP NOT NULL,
    passed          BOOLEAN NOT NULL
);

CREATE TABLE IF NOT EXISTS test_case_execution (
    id              UUID PRIMARY KEY DEFAULT uuidv7(),
    test_run_id     VARCHAR NOT NULL REFERENCES test_run(id),
    suite_name      VARCHAR NOT NULL,
    test_case       VARCHAR NOT NULL,
    exit_code       INTEGER NOT NULL,
    started_at      TIMESTAMP NOT NULL,
    finished_at     TIMESTAMP NOT NULL
);

END_SQL
}

load_wast() {
    local WAST_PATH="${1:?}"
    local WASM_DIR="${2:?}"
    wasm-tools json-from-wast "$WAST_PATH" --wasm-dir "$WASM_DIR" \
        | jq -c '.commands[]'
}

prefix_with_suite_name_and_hash() {
    local SUITE_NAME="${1:?}"
    while read -r LINE; do
        echo "$SUITE_NAME" "$(md5sum <<<"$LINE" | awk 'END { print $1}')" "$LINE"
    done
}

encode_wast_suites() {
    for WAST_PATH in ./testsuite/*.wast; do
        local SUITE_NAME="$(basename $WAST_PATH .wast)"
        local SUITE_DIR="$TEST_STATE_DIR/suites/$SUITE_NAME"
        local WASM_DIR="$SUITE_DIR/modules"
        mkdir -p "$WASM_DIR"
        load_wast "$WAST_PATH" "$WASM_DIR" \
            | ./scripts/encode-wast-cmds.js "$WASM_DIR" 2>>"$SUITE_DIR/skipped.jsonl" \
            | prefix_with_suite_name_and_hash "$SUITE_NAME" >>$SUITE_DIR/commands.txt
    done
}

main() {
    encode_wast_suites
    create_analytics_db
}

main
