FROM rust:1.52.1-slim

WORKDIR /ptunnel

COPY . .

RUN cargo build --release
ENV PATH="/ptunnel/target/release:${PATH}"