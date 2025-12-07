#!/usr/bin/env zsh

TEST_RUN_ID="$1"

if [[ -z "$TEST_RUN_ID" ]]; then
    TEST_RUN_ID="$(ls teststate/runs | sort -r | head -n 1)"
fi

WORK_DIR="teststate/runs/$TEST_RUN_ID"

with_csv_header() {
    echo "$1"
    cat
}

panic_csv() {
    grep -h panicked $WORK_DIR/worker/*/worker.log \
        | awk '{ print $NF }' \
        | with_csv_header "panic"
}

group_by_query() {
    duckdb -table -c "select
        panic,
        count(*) as occurrences
    from read_csv('/dev/stdin')
    group by panic
    order by occurrences desc"
}

panic_csv | group_by_query
