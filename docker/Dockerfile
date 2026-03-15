# Rasa — multi-stage production build on AGNOS
# Stage 1: Build
FROM rust:1.85-bookworm AS builder

RUN apt-get update && apt-get install -y \
    cmake pkg-config libssl-dev git \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .
RUN cargo build --release --workspace

# Stage 2: Runtime on AGNOS
FROM ghcr.io/maccracken/agnosticos:latest

USER root

RUN groupadd -g 1004 rasa && useradd -u 1004 -g rasa -d /var/lib/rasa -s /usr/sbin/nologin rasa

RUN mkdir -p /var/lib/rasa/data \
             /var/lib/rasa/models \
             /etc/rasa \
    && chown -R rasa:rasa /var/lib/rasa /etc/rasa

COPY --from=builder /build/target/release/rasa-mcp /usr/local/bin/rasa-mcp
COPY config/rasa.example.toml /etc/rasa/config.toml

USER rasa
WORKDIR /var/lib/rasa

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["rasa-mcp", "--health"] || exit 1

ENTRYPOINT ["tini", "--"]
CMD ["rasa-mcp"]
