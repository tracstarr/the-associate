# Cross-compile The Associate for Windows (x86_64-pc-windows-gnu) from Linux.
# Output: target/x86_64-pc-windows-gnu/release/assoc.exe

FROM rust:latest

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

# The binary is at:
#   target/x86_64-pc-windows-gnu/release/assoc.exe
