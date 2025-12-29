# syntax=docker/dockerfile:1
FROM rust:1.92-slim-trixie as chef
RUN apt-get update \
 && apt-get install -y --no-install-recommends pkg-config libfuse3-dev git \
 && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef
WORKDIR /usr/src/app/hoppermc

FROM chef as planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef as builder
# Enable native CPU optimizations (SIMD) and other perf flags for ALL build steps
ENV RUSTFLAGS="-C target-cpu=native"
COPY --from=planner /usr/src/app/hoppermc/recipe.json recipe.json
# Build dependencies - this will be cached if recipe.json hasn't changed
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/app/hoppermc/target \
    cargo chef cook --release --recipe-path recipe.json

# Build application
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/usr/src/app/hoppermc/target \
    cargo build --release --workspace && \
    cp target/release/hoppermc /usr/local/bin/hoppermc

# RUNTIME
FROM debian:trixie-slim

RUN apt-get update \
 && apt-get install -y --no-install-recommends fuse3 tini ca-certificates \
 && rm -rf /var/lib/apt/lists/*
RUN sed -i 's/#user_allow_other/user_allow_other/' /etc/fuse.conf

COPY --from=builder /usr/local/bin/hoppermc /usr/local/bin/hoppermc

COPY entrypoint.sh /usr/local/bin/entrypoint.sh
RUN chmod +x /usr/local/bin/entrypoint.sh

ENV RUST_LOG=info

ENTRYPOINT ["/usr/bin/tini", "--", "/usr/local/bin/entrypoint.sh"]
