
# Makefile for building the python-inliner executable


# Rust toolchain
CARGO = cargo

# Targets
BUILD_DIR = target
DEBUG_DIR = $(BUILD_DIR)/debug
RELEASE_DIR = $(BUILD_DIR)/release
TARGET ?= /opt/local//bin

# Executable name
EXECUTABLE = python-inliner

# Default target
all: debug

# Build the project in debug mode
debug:
		$(CARGO) build

# Build the project in release mode
release: test
		$(CARGO) build --release

# Clean up build artifacts
clean:
		$(CARGO) clean

test: debug
	@echo "=== Running Python Inliner Tests ==="
	@echo ""
	@echo "Step 1: Cleaning up previous build..."
	rm -f test/main-inlined.py
	@echo ""
	@echo "Step 2: Running inliner in release mode..."
	PYTHONPATH=test/packages:test/aliens:test $(CARGO) run test/main.py test/main-inlined.py tacos,modules,aliens -r
	@echo ""
	@echo "Step 3: Verifying inlined script produces correct output..."
	@cd test && python3 main-inlined.py > actual_output.txt 2>&1
	@if diff -q test/expected_output.txt test/actual_output.txt > /dev/null 2>&1; then \
		echo "✓ Functional verification PASSED - inlined script output matches expected"; \
	else \
		echo "✗ Functional verification FAILED - output differs:"; \
		diff test/expected_output.txt test/actual_output.txt || true; \
		exit 1; \
	fi
	@echo ""
	@echo "Step 4: Running TYPE_CHECKING test..."
	./test/test_type_checking_bug.sh
	@echo ""
	@echo "Step 5: Running unit tests..."
	$(CARGO) test

install: release
		cp $(RELEASE_DIR)/$(EXECUTABLE) $(TARGET)

# Phony targets
.PHONY: all debug release clean test
