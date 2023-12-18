# Prepare cargo chef
FROM --platform=$BUILDPLATFORM rust:1.71-bullseye AS chef
RUN cargo install cargo-chef 
WORKDIR /app

# Plan the build
FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
RUN cargo build --release --offline

# Runtime
FROM debian:bullseye-slim AS runtime
WORKDIR /app

COPY --from=builder /app/target/release/dsmr dsmr
ENTRYPOINT ["./dsmr"]
