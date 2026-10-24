# syntax=docker/dockerfile:1

FROM rust:1.99-slim-bookworm AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY seeds ./seeds

RUN cargo build --release

FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --create-home --shell /usr/sbin/nologin enem

WORKDIR /app

COPY --from=builder /app/target/release/enem-backend /usr/local/bin/enem-backend

USER enem

ENV BIND_ADDR=0.0.0.0:8080

EXPOSE 8080

HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
    CMD curl -fsS http://127.0.0.1:8080/api/v1/health || exit 1

CMD ["enem-backend"]
