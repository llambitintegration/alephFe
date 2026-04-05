FROM rust:1.82-slim

WORKDIR /app
COPY . .

RUN cargo build 2>&1
RUN cargo test 2>&1

CMD ["cargo", "test"]
