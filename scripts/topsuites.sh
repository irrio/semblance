#!/usr/bin/env zsh

duckdb teststate/analytics.duckdb -c "
with last_run as (
    select id from test_run order by finished_at desc limit 1
)
select
    total_directives - passed_directives as remaining_directives,
    wast_path
from wast_execution
where test_run_id=(select id from last_run)
order by remaining_directives desc
limit 10;
"
