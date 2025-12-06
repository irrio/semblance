#!/usr/bin/env zsh

query () {
duckdb -csv "teststate/analytics.duckdb" <<END_SQL
    select ROW_NUMBER() over () as test_run, sub.passed, sub.failed, sub.panics from (
        with recent_runs as (select id from test_run order by id desc limit 5)
        select
            rr.id as test_run_id,
            sum(case when tce.exit_code = 0 then 1 else 0 end) as passed,
            sum(case when tce.exit_code != 0 then 1 else 0 end) as failed,
            sum(case when tce.exit_code = 101 then 1 else 0 end) as panics,
        from recent_runs rr
        join test_case_execution tce on tce.test_run_id=rr.id
        group by rr.id
        order by rr.id asc
    ) as sub;
END_SQL
}

query | uplot lineplots -d , -H
