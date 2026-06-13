# syntax=docker/dockerfile:1

FROM rust:1-bookworm AS builder
WORKDIR /src

COPY . .

ARG VERSION=dev
RUN cargo build --release -p codux-server

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates tzdata \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --create-home --home-dir /opt/codux-service codux \
    && mkdir -p /data \
    && chown -R codux:codux /data /opt/codux-service

COPY --from=builder /src/target/release/codux-server /usr/local/bin/codux-service
COPY apps/server/deploy/docker.toml /opt/codux-service/config.toml
USER codux
WORKDIR /opt/codux-service
VOLUME ["/data"]
EXPOSE 8088
ENTRYPOINT ["/usr/local/bin/codux-service"]
CMD ["--config", "/opt/codux-service/config.toml"]
