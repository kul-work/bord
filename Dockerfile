FROM rust:latest AS builder

WORKDIR /app
COPY . .

RUN apt-get update && apt-get install -y musl-tools && \
    rustup target add x86_64-unknown-linux-musl

RUN cargo build --release --features perf --target x86_64-unknown-linux-musl
#RUN ls -la /app/target/x86_64-unknown-linux-musl/release/

FROM scratch

COPY --from=builder /app/target/x86_64-unknown-linux-musl/release/main /bord

EXPOSE 80
ENTRYPOINT ["/bord"]
