FROM rust:slim AS base
RUN apt-get update && apt-get install -y --no-install-recommends pkg-config libasound2-dev libx11-dev libxkbcommon-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY . .

# Fetch Marathon 2 scenario data (Bungie limited license, public repo)
# Pinned to a specific commit for reproducible tests
FROM base AS fetch-data
RUN apt-get update && apt-get install -y --no-install-recommends git ca-certificates && rm -rf /var/lib/apt/lists/*
RUN git clone --depth 1 https://github.com/Aleph-One-Marathon/data-marathon-2.git /tmp/m2-data \
 && cd /tmp/m2-data && git fetch --depth 1 origin eaf21a7e9f72706c4c2ff9a2960c4367f739f04d \
 && git checkout eaf21a7e9f72706c4c2ff9a2960c4367f739f04d \
 && dd if=/tmp/m2-data/Map.sceA of=/app/marathon-formats/tests/fixtures/Map bs=128 skip=1 \
 && cp /tmp/m2-data/Shapes.shpA /app/marathon-formats/tests/fixtures/Shapes \
 && cp /tmp/m2-data/Sounds.sndA /app/marathon-formats/tests/fixtures/Sounds \
 && cp "/tmp/m2-data/Physics Models/Standard.phyA" "/app/marathon-formats/tests/fixtures/Physics Model" \
 && rm -rf /tmp/m2-data

FROM fetch-data AS test
RUN cargo build 2>&1
RUN cargo test 2>&1

FROM base AS clippy
RUN rustup component add clippy
RUN cargo clippy -- -D warnings 2>&1

FROM base AS fmt
RUN rustup component add rustfmt
RUN cargo fmt --check 2>&1

FROM fetch-data AS coverage
RUN rustup component add llvm-tools-preview
RUN cargo install cargo-llvm-cov --locked
RUN THRESHOLD=$(cat .coverage-threshold 2>/dev/null || echo 0) && \
    cargo llvm-cov --fail-under-lines "$THRESHOLD" 2>&1

FROM fetch-data AS release
RUN cargo build --release --bin marathon-game 2>&1

CMD ["cargo", "test"]
