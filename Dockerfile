FROM rust:1.52.1-slim as builder

WORKDIR /ptunnel

COPY . .

RUN cargo build --release

FROM ubuntu

COPY --from=builder /ptunnel/target/release /ptunnel
ENV PATH="/ptunnel:${PATH}"