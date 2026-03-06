# ── Build ────────────────────────────────────────────────────────────────────
FROM rust:1.91-bookworm AS builder

WORKDIR /app

# Cache dependencies: copy workspace manifests and create dummy sources
COPY Cargo.toml Cargo.lock ./
COPY gateway/Cargo.toml gateway/Cargo.toml
COPY gateway/build.rs gateway/build.rs
COPY utils/Cargo.toml utils/Cargo.toml
COPY mqtt-client/Cargo.toml mqtt-client/Cargo.toml
COPY mqtt-client/build.rs mqtt-client/build.rs
COPY kafka-producer/Cargo.toml kafka-producer/Cargo.toml
COPY kafka-producer/build.rs kafka-producer/build.rs

RUN mkdir -p gateway/src utils/src mqtt-client/src kafka-producer/src \
    && echo "fn main() {}" > gateway/src/main.rs \
    && echo "fn main() {}" > kafka-producer/src/main.rs \
    && echo "fn main() {}" > mqtt-client/src/main.rs \
    && echo "" > utils/src/lib.rs \
    && cargo build --release --locked -p gateway \
    && rm -r target/release/deps/gateway* target/release/deps/nexus_utils* \
      target/release/.fingerprint/gateway-* target/release/.fingerprint/nexus-utils-*

# Build the real binary
COPY gateway/src gateway/src
COPY gateway/migrations gateway/migrations
COPY gateway/.sqlx gateway/.sqlx
COPY utils/src utils/src

RUN SQLX_OFFLINE=true cargo build --release --locked -p gateway

# ── Runtime ──────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false gateway

COPY --from=builder /app/target/release/gateway /usr/local/bin/gateway

USER gateway

EXPOSE 8000 8001

CMD ["gateway", "run"]
