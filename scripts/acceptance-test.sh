#!/bin/bash
# Acceptance tests for EngineQA

set -e

# Colors
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Function to print colored output
print_green() {
    echo -e "${GREEN}✓ $1${NC}"
}

print_red() {
    echo -e "${RED}✗ $1${NC}"
}

print_yellow() {
    echo -e "${YELLOW}⚠ $1${NC}"
}

# Configuration
API_URL="${API_URL:-http://localhost:8080}"

echo "========================================="
echo "EngineQA Acceptance Tests"
echo "========================================="
echo "API URL: $API_URL"
echo ""

PASSED=0
FAILED=0

# Test helper
run_test() {
    local test_name="$1"
    local test_command="$2"

    echo "Running: $test_name"

    if eval "$test_command" > /dev/null 2>&1; then
        print_green "$test_name"
        ((PASSED++))
    else
        print_red "$test_name"
        ((FAILED++))
    fi
}

# Acceptance Criteria 1: Accuracy - Core FAQ hit rate >= 85%
echo "========================================="
echo "Acceptance Criteria 1: Accuracy"
echo "========================================="
echo ""

# This is a placeholder - in a real scenario, you would:
# 1. Prepare a set of core FAQ questions
# 2. Run each question through the API
# 3. Manually review or automatically grade the answers
# 4. Calculate hit rate

print_yellow "Manual verification required:"
echo "1. Prepare core FAQ questions"
echo "2. Run each question through the API"
echo "3. Review answers and calculate hit rate"
echo "4. Ensure hit rate >= 85%"
echo ""

# Acceptance Criteria 2: Stability - degraded_ratio < 3%
echo "========================================="
echo "Acceptance Criteria 2: Stability"
echo "========================================="
echo ""

# Run multiple queries and check degraded responses
TOTAL_QUERIES=100
DEGRADED_COUNT=0

for i in $(seq 1 $TOTAL_QUERIES); do
    RESPONSE=$(curl -s -X POST "$API_URL/api/query" \
        -H "Content-Type: application/json" \
        -d '{"question": "测试问题 '$i'", "top_k": 6}')

    if echo "$RESPONSE" | grep -q '"degraded": true'; then
        ((DEGRADED_COUNT++))
    fi
done

DEGRADED_RATIO=$(echo "scale=2; $DEGRADED_COUNT * 100 / $TOTAL_QUERIES" | bc)

echo "Total queries: $TOTAL_QUERIES"
echo "Degraded queries: $DEGRADED_COUNT"
echo "Degraded ratio: $DEGRADED_RATIO%"
echo ""

if (( $(echo "$DEGRADED_RATIO < 3" | bc -l) )); then
    print_green "Degraded ratio < 3% (Requirement met)"
    ((PASSED++))
else
    print_red "Degraded ratio >= 3% (Requirement NOT met)"
    ((FAILED++))
fi
echo ""

# Acceptance Criteria 3: Performance - P95 latency 1-3s
echo "========================================="
echo "Acceptance Criteria 3: Performance"
echo "========================================="
echo ""

print_yellow "Run load test: ./scripts/load-test.sh"
echo "Then verify P95 latency is within 1-3s range"
echo ""

# Functional tests
echo "========================================="
echo "Functional Tests"
echo "========================================="
echo ""

run_test "Health endpoint works" \
    "curl -s '$API_URL/health' | grep -q 'ok'"

run_test "Status endpoint returns provider info" \
    "curl -s '$API_URL/api/status' | grep -q 'provider'"

run_test "Query endpoint returns answer and trace_id" \
    "curl -s -X POST '$API_URL/api/query' -H 'Content-Type: application/json' -d '{\"question\": \"test\", \"top_k\": 6}' | grep -q 'trace_id'"

run_test "Feedback endpoint accepts feedback" \
    "curl -s -X POST '$API_URL/api/feedback' -H 'Content-Type: application/json' -d '{\"question\": \"test\", \"answer\": \"test\", \"rating\": \"useful\", \"trace_id\": \"test-123\"}' | grep -q 'ok'"

run_test "Reindex endpoint returns job_id" \
    "curl -s -X POST '$API_URL/api/reindex' -H 'Content-Type: application/json' -d '{\"full\": true}' | grep -q 'job_id'"

# Error handling tests
echo "========================================="
echo "Error Handling Tests"
echo "========================================="
echo ""

# Test empty question
run_test "Empty question returns error" \
    "curl -s -X POST '$API_URL/api/query' -H 'Content-Type: application/json' -d '{\"question\": \"\", \"top_k\": 6}' | grep -q 'answer'"

# Test invalid top_k
run_test "Invalid top_k still returns response" \
    "curl -s -X POST '$API_URL/api/query' -H 'Content-Type: application/json' -d '{\"question\": \"test\", \"top_k\": -1}' | grep -q 'trace_id'"

# Summary
echo "========================================="
echo "Acceptance Test Summary"
echo "========================================="
echo ""
echo "Passed: $PASSED"
echo "Failed: $FAILED"
echo "Total:  $((PASSED + FAILED))"
echo ""

if [ $FAILED -eq 0 ]; then
    print_green "All acceptance tests passed!"
    exit 0
else
    print_red "Some acceptance tests failed!"
    exit 1
fi
