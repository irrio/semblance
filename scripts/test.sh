#!/usr/bin/env bash

SUITES_PASSED=0
SUITES_FAILED=0
SUCCESSES=0
FAILURES=0
SKIPPED=0

for WAST in ./testsuite/*.wast; do
    echo "Running testsuite: $WAST"
    RESULTS=$(./scripts/run-wast.sh "$WAST")
    if [[ $? -eq 0 ]]; then
        ((SUITES_PASSED++))
    else
        ((SUITES_FAILED++))
    fi
    while read -r RESULT; do
        case $RESULT in
            PASSED*)
                NUM_PASSED=${RESULT#"PASSED: "}
                ((SUCCESSES+=NUM_PASSED))
            ;;
            FAILED*)
                NUM_FAILED=${RESULT#"FAILED: "}
                ((FAILURES+=NUM_FAILED))
            ;;
            SKIPPED*)
                NUM_SKIPPED=${RESULT#"SKIPPED: "}
                ((SKIPPED+=NUM_SKIPPED))
            ;;
        esac
    done < <(echo "$RESULTS")
done

echo "-----------------------------"
echo "SUITES PASSED: $SUITES_PASSED"
echo "SUITES FAILED: $SUITES_FAILED"
echo "TOTAL PASSED: $SUCCESSES"
echo "TOTAL FAILED: $FAILURES"
echo "TOTAL SKIPPED: $SKIPPED"
echo "-----------------------------"
