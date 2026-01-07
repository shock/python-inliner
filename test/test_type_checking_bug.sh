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

# Check that TYPE_CHECKING import is present
if ! grep -q "from typing import TYPE_CHECKING" test/test_multiline_type_checking_inlined.py; then
    echo "✗ FAILED: TYPE_CHECKING import missing from output"
    exit 1
fi

# Check that we don't have orphaned import names (the bug)
# Look for lines with just variable names followed by commas with excessive indentation
if grep -q "^        [A-Z_]*_API_KEY,$" test/test_multiline_type_checking_inlined.py; then
    echo "✗ FAILED: Found orphaned import names (indentation bug detected)"
    echo "  This indicates the inliner incorrectly handled multi-line TYPE_CHECKING imports"
    exit 1
fi

# Check for orphaned closing parenthesis
if grep -q "^    )$" test/test_multiline_type_checking_inlined.py; then
    # Make sure it's not part of a valid import statement
    if grep -B1 "^    )$" test/test_multiline_type_checking_inlined.py | grep -q "API_KEY,$"; then
        echo "✗ FAILED: Found orphaned closing parenthesis from incomplete import replacement"
        exit 1
    fi
fi

echo "✓ No orphaned import statements detected"
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
