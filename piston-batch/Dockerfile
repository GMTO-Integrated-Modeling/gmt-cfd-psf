# syntax=docker/dockerfile:1
# Build from the workspace root:
#   docker build -f piston-batch/Dockerfile -t piston-batch .
FROM rust:1.91-slim-bookworm AS builder
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p piston-batch && \
    cp target/release/piston-batch /usr/local/bin/

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/bin/piston-batch /usr/local/bin/piston-batch
ENV RUST_LOG=info
ENTRYPOINT ["piston-batch"]
