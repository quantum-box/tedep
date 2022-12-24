FROM rust:1.66.0-bullseye as builder

RUN apt-get update && apt-get upgrade -y
RUN apt-get install -y curl clang

WORKDIR /app
COPY ./Cargo.toml .
COPY ./Cargo.lock .
COPY ./apps ./apps
COPY ./crates ./crates

RUN --mount=type=cache,target=/root/.cargo \
  --mount=type=cache,target=/root/target \
  cargo build -p tedep-ep --release --target-dir /root/target \
  && cp /root/target/release/tedep-ep /controller 

FROM gcr.io/distroless/cc:nonroot

COPY --from=builder /controller .
