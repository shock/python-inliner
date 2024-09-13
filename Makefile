
# Makefile for building the python-inliner executable


# Rust toolchain
CARGO = cargo

# Targets
BUILD_DIR = target
DEBUG_DIR = $(BUILD_DIR)/debug
RELEASE_DIR = $(BUILD_DIR)/release
TARGET ?= $(HOME)/bin

# Executable name
EXECUTABLE = python-inliner

test:
	@echo $(MY_VAR)

# Default target
all: debug
	@echo "Loaded environment variable: $(HOME)"

# Build the project in debug mode
debug:
		$(CARGO) build
		@echo "Debug build completed. Executable is located at $(DEBUG_DIR)/$(EXECUTABLE)"

# Build the project in release mode
release:
		$(CARGO) build --release
		@echo "Release build completed. Executable is located at $(RELEASE_DIR)/$(EXECUTABLE)"

# Clean up build artifacts
clean:
		$(CARGO) clean
		@echo "Build artifacts cleaned."

install: release
		cp $(RELEASE_DIR)/$(EXECUTABLE) $(TARGET)
		@echo "Executable installed to $(TARGET)"

# Phony targets
.PHONY: all debug release clean test
