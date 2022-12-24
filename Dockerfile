FROM rust:1.66.0-bullseye as builder

RUN apt-get update && apt-get upgrade -y
RUN apt-get install -y curl clang
RUN rustup target add x86_64-unknown-linux-gnu

WORKDIR /app
COPY ./Cargo.toml .
COPY ./Cargo.lock .
COPY ./apps ./apps
COPY ./crates ./crates

RUN --mount=type=cache,target=/root/.cargo \
  --mount=type=cache,target=/root/target \
  cargo build -p tedep-ep --release --target x86_64-unknown-linux-gnu --target-dir /root/target \
  && cp /root/target/x86_64-unknown-linux-gnu/release/tedep-ep /controller

FROM gcr.io/distroless/cc

COPY --from=builder /controller .
