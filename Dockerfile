FROM --platform=$BUILDPLATFORM rust:slim-bookworm AS builder

ARG TARGETARCH

RUN apt-get update && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        dpkg --add-architecture arm64 && \
        apt-get update && \
        apt-get install -y pkg-config libssl-dev gcc-aarch64-linux-gnu libssl-dev:arm64 && \
        rustup target add aarch64-unknown-linux-gnu; \
    else \
        apt-get install -y pkg-config libssl-dev; \
    fi && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /build

COPY Cargo.toml Cargo.lock ./
RUN mkdir src && echo 'fn main() {}' > src/main.rs && \
    echo '' > src/lib.rs && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
        PKG_CONFIG_ALLOW_CROSS=1 \
        cargo build --release --target aarch64-unknown-linux-gnu 2>/dev/null || true; \
    else \
        cargo build --release 2>/dev/null || true; \
    fi && \
    rm -rf src

COPY . .

RUN mkdir -p /build/out && \
    if [ "$TARGETARCH" = "arm64" ]; then \
        CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc \
        PKG_CONFIG_ALLOW_CROSS=1 \
        cargo build --release --all-features --target aarch64-unknown-linux-gnu && \
        aarch64-linux-gnu-strip target/aarch64-unknown-linux-gnu/release/corvo && \
        cp target/aarch64-unknown-linux-gnu/release/corvo /build/out/corvo; \
    else \
        cargo build --release --all-features && \
        strip target/release/corvo && \
        cp target/release/corvo /build/out/corvo; \
    fi

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/out/corvo /usr/local/bin/corvo

RUN corvo --version

ENTRYPOINT ["corvo"]
