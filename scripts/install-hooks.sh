#!/bin/bash
# Install git hooks for jsonb_ivm development

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

echo "📦 Installing git hooks..."

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Install pre-commit hook
if [ -f "$HOOKS_DIR/pre-commit" ]; then
    echo "  → Backing up existing pre-commit hook..."
    mv "$HOOKS_DIR/pre-commit" "$HOOKS_DIR/pre-commit.backup.$(date +%s)"
fi

cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/bash
# Pre-commit hook for jsonb_ivm
# Catches formatting and linting issues before commit

set -e

echo "🔍 Running pre-commit checks..."

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Track if any checks failed
FAILED=0

# Check 1: Rust formatting
echo -n "  → Checking Rust formatting... "
if cargo fmt --check --quiet 2>&1 | grep -q "Diff"; then
    echo -e "${RED}✗${NC}"
    echo -e "${YELLOW}    Run: cargo fmt${NC}"
    FAILED=1
else
    echo -e "${GREEN}✓${NC}"
fi

# Check 2: Clippy lints
echo -n "  → Running clippy... "
if ! cargo clippy --all-targets --all-features --quiet -- -D warnings 2>&1 > /tmp/clippy-output.txt; then
    echo -e "${RED}✗${NC}"
    echo -e "${YELLOW}    Run: cargo clippy --fix --allow-dirty${NC}"
    cat /tmp/clippy-output.txt
    FAILED=1
else
    echo -e "${GREEN}✓${NC}"
fi

# Check 3: Markdown linting (if markdownlint-cli2 is installed)
if command -v markdownlint-cli2 &> /dev/null; then
    echo -n "  → Checking markdown files... "
    if ! markdownlint-cli2 "README.md" "*.md" --quiet 2>&1 > /tmp/markdown-output.txt; then
        echo -e "${RED}✗${NC}"
        echo -e "${YELLOW}    Markdown linting issues found${NC}"
        head -20 /tmp/markdown-output.txt
        # Don't fail on markdown issues, just warn
        echo -e "${YELLOW}    (Warning only - commit will proceed)${NC}"
    else
        echo -e "${GREEN}✓${NC}"
    fi
fi

# Check 4: Build check (quick compile test)
echo -n "  → Checking build... "
if ! cargo check --quiet 2>&1 > /tmp/build-output.txt; then
    echo -e "${RED}✗${NC}"
    echo -e "${YELLOW}    Build errors found${NC}"
    tail -30 /tmp/build-output.txt
    FAILED=1
else
    echo -e "${GREEN}✓${NC}"
fi

# Clean up temp files
rm -f /tmp/clippy-output.txt /tmp/markdown-output.txt /tmp/build-output.txt

# Summary
if [ $FAILED -eq 1 ]; then
    echo ""
    echo -e "${RED}❌ Pre-commit checks failed${NC}"
    echo ""
    echo "Quick fixes:"
    echo "  cargo fmt                          # Fix formatting"
    echo "  cargo clippy --fix --allow-dirty   # Fix clippy issues"
    echo "  cargo build                        # Check build errors"
    echo ""
    echo "Or skip hooks (not recommended): git commit --no-verify"
    exit 1
else
    echo ""
    echo -e "${GREEN}✅ All pre-commit checks passed${NC}"
    exit 0
fi
EOF

chmod +x "$HOOKS_DIR/pre-commit"

echo "✅ Pre-commit hook installed"
echo ""
echo "The hook will run automatically before each commit."
echo "To skip hooks (not recommended): git commit --no-verify"
echo ""
echo "To uninstall: rm $HOOKS_DIR/pre-commit"
