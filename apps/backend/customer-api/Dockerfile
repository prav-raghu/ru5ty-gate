FROM rust:1.97-slim-bookworm AS chef
RUN apt-get update \
    && apt-get install -y --no-install-recommends build-essential cmake pkg-config \
    && rm -rf /var/lib/apt/lists/*
RUN cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY common/ ./common/
COPY apps/backend/ ./apps/backend/
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ENV CARGO_INCREMENTAL=0
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --locked --recipe-path recipe.json -p ru5ty-gate-customer-api
COPY Cargo.toml Cargo.lock ./
COPY common/ ./common/
COPY apps/backend/ ./apps/backend/
RUN cargo build --release --locked -p ru5ty-gate-customer-api --bin customer-api

FROM gcr.io/distroless/cc-debian12:nonroot AS runner
ENV APP_ENV=production
COPY --from=builder /app/target/release/customer-api /usr/local/bin/customer-api
USER nonroot
EXPOSE 4002
HEALTHCHECK --interval=15s --timeout=5s --retries=3 \
  CMD ["/usr/local/bin/customer-api", "healthcheck"]
ENTRYPOINT ["/usr/local/bin/customer-api"]
