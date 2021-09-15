FROM rust:1.52 as builder

WORKDIR /build
COPY . .

RUN apt-get update
RUN apt-get install cmake clang llvm gcc -y
RUN cd /build && cargo build --release

FROM debian:buster-slim
WORKDIR /app

RUN apt-get update
RUN apt install -y libssl-dev

COPY --from=builder /build/target/release/mercury /app/mercury
COPY --from=builder /build/free-space /app/free-space
COPY --from=builder /build/devtools /app/devtools

EXPOSE 8116

CMD mercury -c devtools/config/docker_compose_config.toml


