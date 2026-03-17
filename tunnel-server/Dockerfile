# ── Build ────────────────────────────────────────────────────────────────────
FROM rust:1.91-bookworm AS builder

WORKDIR /app

RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies: copy workspace manifests and create dummy sources
COPY Cargo.toml Cargo.lock ./
COPY tunnel-server/Cargo.toml tunnel-server/Cargo.toml
COPY tunnel-server/build.rs tunnel-server/build.rs
COPY tunnel-client/Cargo.toml tunnel-client/Cargo.toml
COPY tunnel-client/build.rs tunnel-client/build.rs
COPY utils/Cargo.toml utils/Cargo.toml
COPY gateway/Cargo.toml gateway/Cargo.toml
COPY gateway/build.rs gateway/build.rs
COPY mqtt-client/Cargo.toml mqtt-client/Cargo.toml
COPY mqtt-client/build.rs mqtt-client/build.rs
COPY kafka-producer/Cargo.toml kafka-producer/Cargo.toml
COPY kafka-producer/build.rs kafka-producer/build.rs

RUN mkdir -p tunnel-server/src tunnel-client/src utils/src gateway/src mqtt-client/src kafka-producer/src \
    && echo "fn main() {}" > tunnel-server/src/main.rs \
    && echo "fn main() {}" > tunnel-client/src/main.rs \
    && echo "fn main() {}" > gateway/src/main.rs \
    && echo "fn main() {}" > mqtt-client/src/main.rs \
    && echo "fn main() {}" > kafka-producer/src/main.rs \
    && echo "" > utils/src/lib.rs \
    && cargo build --release --locked -p tunnel-server \
    && rm -r target/release/deps/tunnel_server* target/release/deps/nexus_utils* \
      target/release/.fingerprint/tunnel-server-* target/release/.fingerprint/nexus-utils-*

# Build the real binary
COPY tunnel-server/src tunnel-server/src
COPY utils/src utils/src

RUN cargo build --release --locked -p tunnel-server

# ── Runtime ──────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false tunnel-server

COPY --from=builder /app/target/release/tunnel-server /usr/local/bin/tunnel-server
COPY infrastructure/tunnel-server/config.json /etc/tunnel-server/config.json

USER tunnel-server

EXPOSE 8001

CMD ["tunnel-server", "run", "-c", "/etc/tunnel-server/config.json"]
