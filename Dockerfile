# Build stage
FROM rust:1.85-slim AS builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release --package server

# Runtime stage
FROM debian:bookworm-slim
COPY --from=builder /usr/src/app/target/release/server /server
EXPOSE 8080
CMD ["/server"]
