#!/bin/bash
# Load test for EngineQA API

set -e

# Configuration
API_URL="${API_URL:-http://localhost:8080}"
CONCURRENT_REQUESTS="${CONCURRENT_REQUESTS:-50}"
TOTAL_REQUESTS="${TOTAL_REQUESTS:-100}"
OUTPUT_FILE="load-test-results-$(date +%Y%m%d-%H%M%S).json"

echo "========================================="
echo "EngineQA Load Test"
echo "========================================="
echo "API URL: $API_URL"
echo "Concurrent Requests: $CONCURRENT_REQUESTS"
echo "Total Requests: $TOTAL_REQUESTS"
echo "Output File: $OUTPUT_FILE"
echo ""

# Check if ab (Apache Bench) is available
if ! command -v ab &> /dev/null; then
    echo "Error: ab (Apache Bench) is not installed"
    echo "Install it with: sudo apt-get install apache2-utils"
    exit 1
fi

# Test query endpoint
echo "Testing /api/query endpoint..."
echo ""

ab -n $TOTAL_REQUESTS -c $CONCURRENT_REQUESTS \
    -T "application/json" \
    -p load-test-payload.json \
    "$API_URL/api/query" \
    > "$OUTPUT_FILE" 2>&1

echo ""
echo "Load test completed. Results saved to: $OUTPUT_FILE"
echo ""

# Parse and display key metrics
echo "Key Metrics:"
echo "----------------------------------------"

if command -v jq &> /dev/null; then
    # If jq is available, parse JSON output
    TOTAL_TIME=$(grep "Time taken for tests" "$OUTPUT_FILE" | awk '{print $5}')
    RPS=$(grep "Requests per second" "$OUTPUT_FILE" | awk '{print $4}')
    P95=$(grep "95%" "$OUTPUT_FILE" | awk '{print $2}')

    echo "Total Time: ${TOTAL_TIME}s"
    echo "Requests Per Second: ${RPS}"
    echo "95th Percentile: ${P95}ms"
else
    # Fallback: display raw lines
    grep "Time taken for tests" "$OUTPUT_FILE"
    grep "Requests per second" "$OUTPUT_FILE"
    grep "95%" "$OUTPUT_FILE"
fi

echo ""
echo "========================================="

# Check if P95 meets the requirement (1-3s)
P95_MS=$(grep "95%" "$OUTPUT_FILE" | awk '{print $2}' | sed 's/ms//')
if [ ! -z "$P95_MS" ]; then
    P95_INT=$(echo "$P95_MS" | cut -d. -f1)

    if [ $P95_INT -le 3000 ]; then
        echo "✓ P95 latency is within requirement (≤ 3000ms)"
    else
        echo "✗ P95 latency exceeds requirement (> 3000ms)"
        exit 1
    fi
fi
