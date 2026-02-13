#!/bin/bash
# Security checks for EngineQA

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

echo "========================================="
echo "EngineQA Security Check"
echo "========================================="
echo ""

FAILED=0

# Check 1: No tokens in source code
echo "Check 1: No hardcoded tokens in source code"
if grep -r "sk-" . --include="*.rs" --include="*.ts" --include="*.tsx" 2>/dev/null | grep -v "node_modules"; then
    print_red "Hardcoded API keys found"
    FAILED=1
else
    print_green "No hardcoded API keys found"
fi
echo ""

# Check 2: No tokens in logs
echo "Check 2: No tokens in logs"
LOG_FILES=$(find . -name "*.log" -o -name "logs" -type d 2>/dev/null)
if [ -n "$LOG_FILES" ]; then
    if grep -r "INTERNAL_API_TOKEN" logs/ 2>/dev/null; then
        print_red "Tokens found in logs"
        FAILED=1
    else
        print_green "No tokens in logs"
    fi
else
    print_yellow "No log files found to check"
fi
echo ""

# Check 3: No passwords in git history
echo "Check 3: No passwords in git history"
if git log --all --source --full-history -S "password" . 2>/dev/null | grep -q "password"; then
    print_red "Passwords found in git history"
    print_yellow "Run: git filter-repo to remove sensitive data"
    FAILED=1
else
    print_green "No passwords in git history"
fi
echo ""

# Check 4: No full prompts in logs (basic check)
echo "Check 4: No full prompts in logs"
if [ -d "logs" ]; then
    if grep -r "完整 prompt\|full prompt\|system_prompt" logs/ 2>/dev/null; then
        print_red "Full prompts found in logs"
        print_yellow "Ensure prompts are truncated in production"
        FAILED=1
    else
        print_green "No full prompts in logs"
    fi
else
    print_yellow "No log files found to check"
fi
echo ""

# Check 5: .env file not committed
echo "Check 5: .env file not committed"
if git ls-files | grep -q "^\.env$"; then
    print_red ".env file is tracked by git"
    FAILED=1
else
    print_green ".env file not tracked"
fi
echo ""

# Check 6: .env.example exists
echo "Check 6: .env.example exists"
if [ -f ".env.example" ]; then
    print_green ".env.example exists"
else
    print_red ".env.example not found"
    FAILED=1
fi
echo ""

# Check 7: API endpoints require authentication (if applicable)
echo "Check 7: API endpoints require authentication"
print_yellow "Manual check required: Verify production API has authentication"
echo ""

# Check 8: No debug code in production
echo "Check 8: No debug code in production"
if grep -r "println!\|dbg!\|console.log" backend/src frontend/src 2>/dev/null | grep -v "node_modules"; then
    print_red "Debug code found"
    print_yellow "Remove debug statements before production"
    FAILED=1
else
    print_green "No debug code found"
fi
echo ""

# Summary
echo "========================================="
echo "Security Check Summary"
echo "========================================="

if [ $FAILED -eq 0 ]; then
    print_green "All security checks passed!"
    exit 0
else
    print_red "Some security checks failed!"
    exit 1
fi
