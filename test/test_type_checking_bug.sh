#!/usr/bin/env bash
# Test script for TYPE_CHECKING block handling
# Returns 0 on success, 1 on failure

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR/.."

echo "=== Testing TYPE_CHECKING block handling ==="
echo ""

# Clean up previous output
rm -f test/test_multiline_type_checking_inlined.py

# Run the inliner
echo "Running inliner on TYPE_CHECKING test case..."
cargo run --quiet -- test/test_multiline_type_checking.py test/test_multiline_type_checking_inlined.py modules

# Check that output file was created
if [ ! -f test/test_multiline_type_checking_inlined.py ]; then
    echo "✗ FAILED: Inlined output file was not created"
    exit 1
fi

echo "✓ Inlined file created"
echo ""

# Validate the output
echo "Validating inlined output..."

# The correct behavior is to STRIP TYPE_CHECKING blocks entirely
# TYPE_CHECKING is always False at runtime, so these blocks should not be in the inlined file

# Check that TYPE_CHECKING blocks were removed (should NOT contain "if TYPE_CHECKING:")
if grep -q "if TYPE_CHECKING:" test/test_multiline_type_checking_inlined.py; then
    echo "✗ FAILED: Found TYPE_CHECKING block in output (should be stripped)"
    exit 1
fi

echo "✓ TYPE_CHECKING blocks stripped from output"

# Check that we don't have orphaned import names (the bug from before the fix)
if grep -q "^        [A-Z_]*_API_KEY,$" test/test_multiline_type_checking_inlined.py; then
    echo "✗ FAILED: Found orphaned import names with excessive indentation"
    echo "  This indicates incomplete import replacement"
    exit 1
fi

echo "✓ No orphaned import statements detected"

# Check that the module import statement from TYPE_CHECKING block was removed
if grep -q "from modules.environment import" test/test_multiline_type_checking_inlined.py; then
    echo "✗ FAILED: Found import from TYPE_CHECKING block in output (should be stripped)"
    exit 1
fi

echo "✓ TYPE_CHECKING imports properly removed"
echo ""

# Try to run the inlined file
echo "Running inlined Python file..."
if python test/test_multiline_type_checking_inlined.py > /tmp/type_checking_output.txt 2>&1; then
    OUTPUT=$(cat /tmp/type_checking_output.txt)
    if [ "$OUTPUT" = "Provider: LiteLLM Provider" ]; then
        echo "✓ Inlined file executed successfully with correct output"
        echo ""
        echo "=== TYPE_CHECKING test PASSED ==="
        exit 0
    else
        echo "✗ FAILED: Inlined file ran but produced unexpected output:"
        echo "  Expected: 'Provider: LiteLLM Provider'"
        echo "  Got: '$OUTPUT'"
        exit 1
    fi
else
    ERROR=$(cat /tmp/type_checking_output.txt)
    echo "✗ FAILED: Inlined file failed to execute:"
    echo "$ERROR" | head -10
    echo ""
    echo "This is the known bug described in TODO.md"
    echo "TYPE_CHECKING blocks are incorrectly inlined, causing IndentationError"
    exit 1
fi
