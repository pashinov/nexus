# ── Build ────────────────────────────────────────────────────────────────────
FROM rust:1.91-bookworm AS builder

WORKDIR /app

# Cache dependencies: copy manifests first, build a dummy binary
COPY Cargo.toml Cargo.lock build.rs ./
RUN mkdir src && echo "fn main() {}" > src/main.rs \
 && cargo build --release --locked \
 && rm -f target/release/deps/gateway*

# Build the real binary
COPY src ./src
COPY migrations ./migrations
COPY .sqlx ./.sqlx
RUN SQLX_OFFLINE=true cargo build --release --locked

# ── Runtime ──────────────────────────────────────────────────────────────────
FROM debian:bookworm-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends ca-certificates libssl3 \
 && rm -rf /var/lib/apt/lists/*

RUN useradd -r -s /bin/false gateway

COPY --from=builder /app/target/release/gateway /usr/local/bin/gateway

USER gateway

EXPOSE 8000

CMD ["gateway", "run"]
