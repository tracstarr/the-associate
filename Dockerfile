# Cross-compile The Associate for Windows (x86_64-pc-windows-gnu) from Linux.
#
# USAGE — release binary copied to local target/ folder:
#   docker build -t assoc-build --target builder .
#   docker build --target export --output "type=local,dest=target/x86_64-pc-windows-gnu/release" .
#
# Or use the build script:
#   ./build.sh
#
# Output: target/x86_64-pc-windows-gnu/release/assoc.exe

# ── Stage 1: builder ────────────────────────────────────────────────────────
FROM rust:latest AS builder

# Install MinGW-w64 cross-compiler for the Windows GNU target
RUN apt-get update && apt-get install -y \
    mingw-w64 \
    && rm -rf /var/lib/apt/lists/*

# Add the Windows GNU cross-compilation target
RUN rustup target add x86_64-pc-windows-gnu

WORKDIR /app

# Copy manifest first for layer caching of dependency downloads
COPY Cargo.toml Cargo.lock ./
# Stub src so cargo can fetch deps without full source
RUN mkdir src && echo 'fn main(){}' > src/main.rs
RUN cargo fetch --target x86_64-pc-windows-gnu
RUN rm -rf src

# Copy full source
COPY . .

# Build release binary for Windows
RUN cargo build --release --target x86_64-pc-windows-gnu

# ── Stage 2: export ──────────────────────────────────────────────────────────
# Minimal stage used only to copy the binary out via BuildKit --output.
# Run: docker build --target export --output "type=local,dest=target/x86_64-pc-windows-gnu/release" .
FROM scratch AS export
COPY --from=builder /app/target/x86_64-pc-windows-gnu/release/assoc.exe /assoc.exe
