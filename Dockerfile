FROM rust:1.66.0-bullseye as builder

RUN apt-get update && apt-get upgrade -y
RUN apt-get install -y curl clang
RUN curl -Lo mold.tar.gz https://github.com/rui314/mold/releases/download/v1.7.1/mold-1.7.1-x86_64-linux.tar.gz \
  && tar xvzf mold.tar.gz -C /usr/local --strip-components=1
RUN curl -Lo clang+llvm.tar.xz https://github.com/llvm/llvm-project/releases/download/llvmorg-15.0.0/clang+llvm-15.0.0-aarch64-linux-gnu.tar.xz \
  && tar xvJf clang+llvm.tar.xz -C /usr/local --strip-components=1

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
