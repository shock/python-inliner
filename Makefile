
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

# Default target
all: debug

# Build the project in debug mode
debug:
		$(CARGO) build

# Build the project in release mode
release:
		$(CARGO) build --release

# Clean up build artifacts
clean:
		$(CARGO) clean

test: debug
	$(DEBUG_DIR)/$(EXECUTABLE) test/main.py test/main-inlined.py

install: release
		cp $(RELEASE_DIR)/$(EXECUTABLE) $(TARGET)

# Phony targets
.PHONY: all debug release clean test
