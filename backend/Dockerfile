# syntax=docker/dockerfile:1

FROM rust:1.98.1-slim-trixie AS builder
WORKDIR /src
COPY . .
# target/ lives in a cache mount, so the binaries must be copied out in the same step. The seeder
# is for the demo's opt-in `seed` service.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked --bin api --bin worker --bin migrate --bin seed \
    && cp target/release/api target/release/worker target/release/migrate target/release/seed \
        /usr/local/bin/

FROM debian:trixie-slim
COPY --from=builder /usr/local/bin/api /usr/local/bin/worker /usr/local/bin/migrate \
    /usr/local/bin/seed /usr/local/bin/
# Numeric so Kubernetes `runAsNonRoot` can verify it without a passwd entry.
USER 10001:10001
EXPOSE 3000
CMD ["api"]
