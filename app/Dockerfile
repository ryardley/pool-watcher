# Build context = repo root: the app crate path-depends on
# scanner/crates/handle, so both trees must be in the context.
#
#   docker build -f app/Dockerfile .

# Stage 1: build the browser seal bundle (app/static/seal.js).
FROM node:22-alpine AS web
WORKDIR /src/app/web
COPY app/web/package.json app/web/package-lock.json ./
RUN npm ci
COPY app/web/seal.ts ./
RUN npm run build

# Stage 2: build the Rust binary. The seal bundle is copied in before the
# build because web.rs embeds it with include_str!.
FROM rust:1-bookworm AS build
WORKDIR /app
COPY app/Cargo.toml app/Cargo.lock ./
COPY app/src ./src
COPY app/static/index.html ./static/index.html
COPY --from=web /src/app/static/seal.js ./static/seal.js
COPY scanner/crates/handle /scanner/crates/handle
RUN cargo build --release --locked

# Stage 3: slim runtime with only the binary and static assets.
FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=build /app/target/release/app /usr/local/bin/app
COPY --from=build /app/static ./static
RUN mkdir -p /data/.state /ledger /run/scanner

ENV RPC_URL=http://host.docker.internal:8545 \
    WS_URL=ws://host.docker.internal:8545 \
    CHAIN_ID=31337 \
    SCANNER_SOCKET=/run/scanner/scanner-b.sock \
    DEPLOYMENT_PATH=/data/.state/deployment.json \
    LEDGER_PATH=/ledger/ledger.beancount \
    BIND_ADDR=0.0.0.0:8080

EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/app"]
