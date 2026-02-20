# Contributing to The Associate

## Installing

```powershell
irm https://raw.githubusercontent.com/tracstarr/the-associate/main/install.ps1 | iex
```

Downloads `assoc.exe` from the latest GitHub release and installs to `%LOCALAPPDATA%\Programs\assoc` on your PATH.

## Prerequisites (for development)

- Rust toolchain (`stable-x86_64-pc-windows-gnu`) via [rustup](https://rustup.rs/)
- [MSYS2](https://www.msys2.org/) with MinGW64 (`C:\msys64\mingw64\bin` on PATH) — required for the GNU linker
- Docker — required for the cross-compilation build script

## Building

**Local build** (Windows with MSYS2):

```bash
export PATH="/c/Users/$USER/.cargo/bin:$PATH:/c/msys64/mingw64/bin"
cargo build --release
```

**Docker build** (cross-compile from Linux or any platform with Docker):

```bash
./build.sh
```

This produces `assoc.exe` in the project root.

## Testing & Linting

```bash
cargo test
cargo clippy
cargo fmt -- --check
```

All three must pass before a PR will be merged. The CI workflow runs them automatically on every pull request.

## Submitting a Pull Request

1. Fork the repository and create a branch from `main`.
2. Make your changes and ensure tests and lints pass locally.
3. Open a PR against `main`. The CI workflow will run automatically.

## Releasing

Releases are triggered by pushing a version tag. The tag must match the version in `Cargo.toml`:

```bash
# 1. Bump version in Cargo.toml
# 2. Commit and merge to main
# 3. Tag and push
git tag v0.2.0
git push origin v0.2.0
```

The release workflow builds `assoc.exe` via Docker and publishes a GitHub release with the binary attached.
