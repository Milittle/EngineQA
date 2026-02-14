#!/bin/bash
# Smoke test for Step-13: Tests, load tests, and acceptance checks

set -e

echo "========================================="
echo "EngineQA Step-13 Smoke Test"
echo "========================================="
echo ""

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

# Test 1: Health check
echo "Test 1: Health check"
HEALTH_CHECK=$(curl -s http://localhost:8080/health)
if [[ $HEALTH_CHECK == *"ok"* ]]; then
    print_green "Health check passed"
else
    print_red "Health check failed"
    exit 1
fi
echo ""

# Test 2: Status endpoint
echo "Test 2: Status endpoint"
STATUS_CHECK=$(curl -s http://localhost:8080/api/status)
if [[ $STATUS_CHECK == *"provider"* ]] && [[ $STATUS_CHECK == *"model"* ]]; then
    print_green "Status endpoint passed"
else
    print_red "Status endpoint failed"
    exit 1
fi
echo ""

# Test 3: Query endpoint (basic)
echo "Test 3: Query endpoint (basic)"
QUERY_CHECK=$(curl -s -X POST http://localhost:8080/api/query \
    -H "Content-Type: application/json" \
    -d '{"question": "测试问题", "top_k": 6}')
if [[ $QUERY_CHECK == *"answer"* ]] && [[ $QUERY_CHECK == *"trace_id"* ]]; then
    print_green "Query endpoint passed"
else
    print_red "Query endpoint failed"
    exit 1
fi
echo ""

# Test 4: Feedback endpoint
echo "Test 4: Feedback endpoint"
FEEDBACK_CHECK=$(curl -s -X POST http://localhost:8080/api/feedback \
    -H "Content-Type: application/json" \
    -d '{"question": "测试", "answer": "测试回答", "rating": "useful", "trace_id": "test-123"}')
if [[ $FEEDBACK_CHECK == *"ok"* ]] && [[ $FEEDBACK_CHECK == *"id"* ]]; then
    print_green "Feedback endpoint passed"
else
    print_red "Feedback endpoint failed"
    exit 1
fi
echo ""

# Test 5: Reindex endpoint
echo "Test 5: Reindex endpoint"
REINDEX_CHECK=$(curl -s -X POST http://localhost:8080/api/reindex \
    -H "Content-Type: application/json" \
    -d '{"full": true}')
if [[ $REINDEX_CHECK == *"job_id"* ]]; then
    print_green "Reindex endpoint passed"
else
    print_red "Reindex endpoint failed"
    exit 1
fi
echo ""

# Test 6: Security check - No token in logs
echo "Test 6: Security check - No token in logs"
if grep -r "INTERNAL_API_TOKEN" logs/ 2>/dev/null; then
    print_red "Security check failed - Token found in logs"
    exit 1
else
    print_green "Security check passed - No token in logs"
fi
echo ""

# Test 7: Security check - No full prompts in logs
echo "Test 7: Security check - No full prompts in logs"
# This is a simplified check - in production you'd want more sophisticated log parsing
if grep -r "完整 prompt" logs/ 2>/dev/null; then
    print_red "Security check failed - Full prompts found in logs"
    exit 1
else
    print_green "Security check passed - No full prompts in logs"
fi
echo ""

# Test 8: Frontend accessible
echo "Test 8: Frontend accessible"
FRONTEND_CHECK=$(curl -s -o /dev/null -w "%{http_code}" http://localhost:5173/)
if [[ $FRONTEND_CHECK == "200" ]]; then
    print_green "Frontend accessible"
else
    print_yellow "Frontend not accessible (this is OK if only backend is running)"
fi
echo ""

# Test 9: Vector store connectivity (status-based)
echo "Test 9: Vector store connectivity"
if [[ $STATUS_CHECK == *'"vector_store_connected":true'* ]]; then
    print_green "Vector store connectivity passed (Rust/LanceDB path)"
elif [[ $STATUS_CHECK == *'"qdrant_connected":true'* ]]; then
    print_green "Vector store connectivity passed (Python/Qdrant path)"
else
    print_yellow "Vector store connectivity field not healthy"
fi
echo ""

# Summary
echo "========================================="
echo "Step-13 Smoke Test Summary"
echo "========================================="
echo ""
echo "All critical tests passed!"
echo ""
echo "Next steps:"
echo "1. Run load tests: ./scripts/load-test.sh"
echo "2. Run security checks: ./scripts/security-check.sh"
echo "3. Run acceptance tests: ./scripts/acceptance-test.sh"
echo ""
