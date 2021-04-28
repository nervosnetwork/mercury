FROM rust:1.51

WORKDIR /usr/src/app
COPY . .

RUN cargo install --path .

EXPOSE 8116




