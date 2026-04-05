FROM rust:1.82-slim AS base
WORKDIR /app
COPY . .

FROM base AS test
RUN cargo build 2>&1
RUN cargo test 2>&1

FROM base AS clippy
RUN rustup component add clippy
RUN cargo clippy -- -D warnings 2>&1

FROM base AS fmt
RUN rustup component add rustfmt
RUN cargo fmt --check 2>&1

FROM base AS coverage
RUN cargo install cargo-tarpaulin --locked
RUN THRESHOLD=$(cat .coverage-threshold 2>/dev/null || echo 0) && \
    cargo tarpaulin --out stdout --fail-under "$THRESHOLD" 2>&1

CMD ["cargo", "test"]
