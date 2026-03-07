# ── Build ────────────────────────────────────────────────────────────────────
FROM rust:1.91-bookworm AS builder

WORKDIR /app

# cmake + C compiler required for rdkafka cmake-build feature
RUN apt-get update && apt-get install -y --no-install-recommends \
    cmake \
    libssl-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

# Cache dependencies: copy workspace manifests and create dummy sources
COPY Cargo.toml Cargo.lock ./
COPY kafka-producer/Cargo.toml kafka-producer/Cargo.toml
COPY kafka-producer/build.rs kafka-producer/build.rs
COPY gateway/Cargo.toml gateway/Cargo.toml
COPY gateway/build.rs gateway/build.rs
COPY utils/Cargo.toml utils/Cargo.toml
COPY mqtt-client/Cargo.toml mqtt-client/Cargo.toml
COPY mqtt-client/build.rs mqtt-client/build.rs

RUN mkdir -p kafka-producer/src gateway/src utils/src mqtt-client/src \
    && echo "fn main() {}" > kafka-producer/src/main.rs \
    && echo "fn main() {}" > gateway/src/main.rs \
    && echo "fn main() {}" > mqtt-client/src/main.rs \
    && echo "" > utils/src/lib.rs \
    && cargo build --release --locked -p kafka-producer \
    && rm -r target/release/deps/kafka_producer* target/release/deps/nexus_utils* \
      target/release/.fingerprint/kafka-producer-* target/release/.fingerprint/nexus-utils-*

# Build the real binary
COPY kafka-producer/src kafka-producer/src
COPY utils/src utils/src

RUN cargo build --release --locked -p kafka-producer

# ── Runtime ──────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false kafka-producer \
    && mkdir -p /var/lib/kafka-producer \
    && chown kafka-producer /var/lib/kafka-producer

COPY --from=builder /app/target/release/kafka-producer /usr/local/bin/kafka-producer
COPY infrastructure/kafka-producer/config.json /etc/kafka-producer/config.json

USER kafka-producer

CMD ["kafka-producer", "run"]
