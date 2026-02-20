#!/usr/bin/env bash
set -euo pipefail

# The Associate installer
# Builds the release binary and installs to ~/.local/bin
#
# Usage:
#   ./install.sh              # Build and install (or update)
#   ./install.sh update       # Pull latest source, rebuild, and install
#   ./install.sh uninstall    # Remove binary from ~/.local/bin

BINARY_NAME="assoc"
INSTALL_DIR="${HOME}/.local/bin"
ACTION="${1:-install}"

# Detect platform and set PATH for build tools
if [[ "$(uname -o 2>/dev/null)" == "Msys" ]] || [[ "$OSTYPE" == "msys" ]]; then
    # Windows/MSYS2 - need MinGW on PATH
    export PATH="${HOME}/.cargo/bin:${PATH}:/c/msys64/mingw64/bin"
    BINARY_NAME="assoc.exe"
fi

INSTALLED_PATH="${INSTALL_DIR}/${BINARY_NAME}"

add_to_path() {
    if echo "$PATH" | tr ':' '\n' | grep -q "$(realpath "$INSTALL_DIR" 2>/dev/null || echo "$INSTALL_DIR")"; then
        return
    fi

    echo ""
    echo "WARNING: ${INSTALL_DIR} is not on your PATH."
    echo ""

    # Detect shell and suggest the right profile
    SHELL_NAME="$(basename "$SHELL" 2>/dev/null || echo "bash")"
    case "$SHELL_NAME" in
        zsh)  PROFILE="$HOME/.zshrc" ;;
        bash)
            if [[ -f "$HOME/.bash_profile" ]]; then
                PROFILE="$HOME/.bash_profile"
            else
                PROFILE="$HOME/.bashrc"
            fi
            ;;
        *)    PROFILE="$HOME/.profile" ;;
    esac

    read -rp "Add ${INSTALL_DIR} to PATH in ${PROFILE}? [Y/n] " answer
    if [[ "${answer:-Y}" =~ ^[Yy]$ ]]; then
        echo "" >> "$PROFILE"
        echo "# The Associate" >> "$PROFILE"
        echo "export PATH=\"\${HOME}/.local/bin:\${PATH}\"" >> "$PROFILE"
        echo "Added to ${PROFILE}. Run 'source ${PROFILE}' or restart your shell."
    else
        echo "Skipped. Add manually:"
        echo "  export PATH=\"\${HOME}/.local/bin:\${PATH}\""
    fi
}

# === UNINSTALL ===
if [[ "$ACTION" == "uninstall" ]]; then
    echo "Uninstalling The Associate..."

    if [[ -f "$INSTALLED_PATH" ]]; then
        rm "$INSTALLED_PATH"
        echo "Removed ${INSTALLED_PATH}"
        echo ""
        echo "The Associate has been uninstalled."
        echo "Note: Your shell profile may still have a PATH entry for ${INSTALL_DIR}."
        echo "Remove it manually if no other binaries use that directory."
    else
        echo "The Associate is not installed at ${INSTALLED_PATH}."
    fi
    exit 0
fi

# === UPDATE ===
if [[ "$ACTION" == "update" ]]; then
    echo "Updating The Associate..."

    # Pull latest source if inside a git repository
    if git rev-parse --is-inside-work-tree &>/dev/null; then
        echo "Pulling latest source..."
        git pull
    else
        echo "Not a git repository — skipping source update. Build from current source."
    fi
fi

# === INSTALL / UPDATE (build & copy) ===
if [[ "$ACTION" == "install" || "$ACTION" == "update" ]]; then
    IS_UPDATE=false
    if [[ -f "$INSTALLED_PATH" ]]; then
        IS_UPDATE=true
    fi

    echo "Building The Associate..."
    cargo build --release

    # Create install directory
    mkdir -p "$INSTALL_DIR"

    # Copy binary
    BUILT="target/release/${BINARY_NAME}"
    if [[ ! -f "$BUILT" ]]; then
        echo "Error: Build succeeded but binary not found at $BUILT"
        exit 1
    fi

    cp "$BUILT" "$INSTALLED_PATH"
    echo "Installed ${BINARY_NAME} to ${INSTALL_DIR}/"

    # Check if INSTALL_DIR is on PATH
    add_to_path

    echo ""
    if [[ "$IS_UPDATE" == true ]]; then
        echo "Done! The Associate has been updated."
    else
        echo "Done! Run 'assoc' to start The Associate."
    fi
else
    echo "Unknown action: $ACTION"
    echo "Usage: $0 [install|update|uninstall]"
    exit 1
fi
