# Segment piston container, built from the workspace root by deploy-piston-batch.sh:
#   docker build -f piston-batch/Dockerfile -t piston-batch .
FROM rust:1.91-slim-bookworm AS builder
RUN apt-get update && \
    apt-get install -y --no-install-recommends build-essential pkg-config && \
    rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN cargo build --release --locked -p piston-batch --bin piston-batch && \
    cp target/release/piston-batch /usr/local/bin/

FROM debian:bookworm-slim
RUN apt-get update && \
    apt-get install -y --no-install-recommends ca-certificates && \
    rm -rf /var/lib/apt/lists/*
COPY --from=builder /usr/local/bin/piston-batch /usr/local/bin/piston-batch
ENV RUST_LOG=info
ENTRYPOINT ["piston-batch"]
