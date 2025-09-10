# syntax=docker/dockerfile:1

# Build stage: Debian Bookworm cross-compile to aarch64-unknown-linux-gnu
FROM debian:bookworm AS builder

ENV DEBIAN_FRONTEND=noninteractive

# Base toolchain and cross-compilation deps
RUN dpkg --add-architecture arm64 && \
    apt-get update && \
    apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    curl \
    ca-certificates \
    git \
    gcc-aarch64-linux-gnu \
    libc6-dev-arm64-cross \
    clang \
    libclang-dev \
    libv4l-dev \
    libcups2-dev:arm64 \
    libgphoto2-dev:arm64 \
    && ldconfig && rm -rf /var/lib/apt/lists/*

# Install Rust via rustup
ENV RUSTUP_HOME=/root/.rustup \
    CARGO_HOME=/root/.cargo \
    PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin

RUN curl -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
RUN rustup target add aarch64-unknown-linux-gnu

# Cross-compile environment
ENV PKG_CONFIG_ALLOW_CROSS=1 \
    CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
    CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc \
    AR_aarch64_unknown_linux_gnu=aarch64-linux-gnu-ar \
    PKG_CONFIG_PATH=/usr/lib/aarch64-linux-gnu/pkgconfig \
    PKG_CONFIG_LIBDIR=/usr/lib/aarch64-linux-gnu/pkgconfig

# Prepare workspace
WORKDIR /work/canon_test_cam

# Cache dependencies first
COPY canon_test_cam/Cargo.toml ./Cargo.toml
COPY canon_test_cam/Cargo.lock ./Cargo.lock
COPY canon_test_cam/build.rs ./build.rs
# Create empty src to allow cargo to resolve deps if needed (safe no-op if not used)
RUN mkdir -p src && [ -f src/main.rs ] || echo "fn main(){}" > src/main.rs
RUN cargo fetch

# Now copy actual sources
COPY canon_test_cam/src ./src
COPY canon_test_cam/html ./html
COPY canon_test_cam/migrations ./migrations

# Build release for aarch64-unknown-linux-gnu
RUN cargo build --release --target aarch64-unknown-linux-gnu

# Collect artifact
RUN mkdir -p /out && \
    cp target/aarch64-unknown-linux-gnu/release/canon_test_cam /out/cam_test

# Final minimal image that contains only the artifact for easy docker cp
FROM scratch AS artifact
COPY --from=builder /out/cam_test /cam_test
CMD ["/cam_test"]
