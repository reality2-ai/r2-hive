# Build an R2 wayfinder (r2-hive) container image.
#
# The R2 protocol crates are git-pinned to r2-core in Cargo.toml. For an
# OFFLINE image build (no r2-core git credentials in the build), this file
# COPYs a sibling r2-core checkout and activates the Cargo `[patch]` block so
# cargo compiles against that local copy. The build context must therefore
# contain BOTH repos — build from the PARENT directory holding them side by side:
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
# Activate the [patch] block (present, commented, at the bottom of Cargo.toml) so cargo
# builds against the COPY-ed local ../r2-core instead of fetching the git-pinned r2-core
# (offline; no r2-core git credentials needed in the build).
RUN sed -i 's|^# \(\[patch\)|\1|; s|^# \(r2-[a-z0-9-]* = { path = "\.\./r2-core/crates/\)|\1|' Cargo.toml
RUN cargo build --release --bin r2-hive

FROM debian:trixie-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/r2-hive/target/release/r2-hive /usr/local/bin/
EXPOSE 21042
# Bind 0.0.0.0 inside the container only by explicit opt-in; /r2/mgmt stays
# disabled on non-loopback listeners. Put a TLS terminator (Caddy/nginx) in
# front for wss:// in production.
CMD ["r2-hive", "--bind", "0.0.0.0", "--allow-public-bind", "--port", "21042", "--no-usb"]
