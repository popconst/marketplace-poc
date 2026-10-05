FROM rust:1.98.1-slim-trixie AS builder
WORKDIR /src
COPY . .
# Plain RUN steps only, so the image builds with or without BuildKit. The seeder is for the demo's
# opt-in `seed` service.
RUN cargo build --release --locked --bin api --bin worker --bin migrate --bin seed

FROM debian:trixie-slim
COPY --from=builder /src/target/release/api /src/target/release/worker \
    /src/target/release/migrate /src/target/release/seed /usr/local/bin/
# Numeric so Kubernetes `runAsNonRoot` can verify it without a passwd entry.
USER 10001:10001
EXPOSE 3000
CMD ["api"]
