# Build an R2 wayfinder (r2-hive) container image.
#
# The R2 protocol crates are path-dependencies on a sibling r2-core
# checkout, so the build context must contain BOTH repos. Build from the
# PARENT directory that holds r2-hive/ and r2-core/ side by side:
#
#   git clone https://github.com/reality2-ai/r2-core.git
#   git clone https://github.com/reality2-ai/r2-hive.git
#   docker build -f r2-hive/Dockerfile -t r2-hive .
#
# (Once the crates are published to crates.io this can become a
# self-contained single-repo build.)

FROM rust:1-slim AS builder
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libssl-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY r2-core/ r2-core/
COPY r2-hive/ r2-hive/
WORKDIR /build/r2-hive
RUN cargo build --release --bin r2-hive

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/r2-hive/target/release/r2-hive /usr/local/bin/
EXPOSE 21042
# Bind 0.0.0.0 inside the container; put a TLS terminator (Caddy/nginx)
# in front for wss:// in production.
CMD ["r2-hive", "--bind", "0.0.0.0", "--port", "21042", "--no-usb"]
