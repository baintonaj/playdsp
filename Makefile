# playdsp Makefile
#
# Targets:
#   all            Build release binary (default)
#   build          Build debug binary
#   release        Build optimised release binary
#   install        Copy release binary to DESTDIR
#   install-cargo  Install via `cargo install` (simplest cross-platform option)
#   uninstall      Remove binary from DESTDIR
#   clean          Remove Cargo build artifacts
#   help           Show this message
#
# Variables:
#   DESTDIR        Install directory. Defaults:
#                    Unix : /usr/local/bin
#                    Windows (Git Bash / MSYS2): $(USERPROFILE)/.cargo/bin
#
# Examples:
#   make                              # release build
#   make install                      # install to default location
#   make install DESTDIR=$(HOME)/.local/bin
#   sudo make install DESTDIR=/usr/local/bin
#   make install-cargo                # always works, no DESTDIR needed

# ---------------------------------------------------------------------------
# Platform detection
# ---------------------------------------------------------------------------

ifdef OS
    # Windows — requires Git Bash, MSYS2, or another POSIX-compatible shell
    EXE_SUFFIX  := .exe
    DESTDIR     ?= $(USERPROFILE)/.cargo/bin
    _MKDIR      := mkdir -p
    _CP         := cp -f
    _RM         := rm -f
else
    EXE_SUFFIX  :=
    DESTDIR     ?= /usr/local/bin
    _MKDIR      := mkdir -p
    _CP         := cp -f
    _RM         := rm -f
endif

# ---------------------------------------------------------------------------
# Paths
# ---------------------------------------------------------------------------

BIN         := playdsp$(EXE_SUFFIX)
RELEASE_BIN := target/release/$(BIN)
DEBUG_BIN   := target/debug/$(BIN)

# ---------------------------------------------------------------------------
# Targets
# ---------------------------------------------------------------------------

.PHONY: all build release install install-cargo uninstall clean help

all: release

build:
	cargo build

release:
	cargo build --release

install: release
	$(_MKDIR) "$(DESTDIR)"
	$(_CP) "$(RELEASE_BIN)" "$(DESTDIR)/$(BIN)"
	@echo "Installed: $(DESTDIR)/$(BIN)"

# Universal install — handles .exe suffix and PATH automatically.
# Recommended for Windows users without a POSIX shell.
install-cargo:
	cargo install --path . --force

uninstall:
	$(_RM) "$(DESTDIR)/$(BIN)"
	@echo "Removed:   $(DESTDIR)/$(BIN)"

clean:
	cargo clean

help:
	@echo ""
	@echo "Usage: make [target] [DESTDIR=/install/path]"
	@echo ""
	@echo "Targets:"
	@echo "  all            Build release binary (default)"
	@echo "  build          Build debug binary"
	@echo "  release        Build optimised release binary"
	@echo "  install        Copy release binary to DESTDIR"
	@echo "  install-cargo  Install via 'cargo install' (cross-platform, no DESTDIR needed)"
	@echo "  uninstall      Remove binary from DESTDIR"
	@echo "  clean          Remove Cargo build artifacts"
	@echo "  help           Show this message"
	@echo ""
	@echo "Current DESTDIR: $(DESTDIR)"
	@echo "Binary name:     $(BIN)"
	@echo ""
	@echo "Examples:"
	@echo "  make install"
	@echo "  make install DESTDIR=\$$HOME/.local/bin"
	@echo "  sudo make install DESTDIR=/usr/local/bin"
	@echo "  make install-cargo"
	@echo ""
