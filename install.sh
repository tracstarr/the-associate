#!/usr/bin/env bash
set -euo pipefail

# The Associate installer
# Builds the release binary and installs to ~/.local/bin

BINARY_NAME="assoc"
INSTALL_DIR="${HOME}/.local/bin"

echo "Building The Associate..."

# Detect platform and set PATH for build tools
if [[ "$(uname -o 2>/dev/null)" == "Msys" ]] || [[ "$OSTYPE" == "msys" ]]; then
    # Windows/MSYS2 - need MinGW on PATH
    export PATH="${HOME}/.cargo/bin:${PATH}:/c/msys64/mingw64/bin"
    BINARY_NAME="assoc.exe"
fi

# Build release binary
cargo build --release

# Create install directory
mkdir -p "$INSTALL_DIR"

# Copy binary
BUILT="target/release/${BINARY_NAME}"
if [[ ! -f "$BUILT" ]]; then
    echo "Error: Build succeeded but binary not found at $BUILT"
    exit 1
fi

cp "$BUILT" "${INSTALL_DIR}/${BINARY_NAME}"
echo "Installed ${BINARY_NAME} to ${INSTALL_DIR}/"

# Check if INSTALL_DIR is on PATH
if ! echo "$PATH" | tr ':' '\n' | grep -q "$(realpath "$INSTALL_DIR")"; then
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
fi

echo ""
echo "Done! Run 'assoc' to start The Associate."
