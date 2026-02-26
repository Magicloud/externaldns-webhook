# syntax=docker/dockerfile:1.7-labs

FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev g++ clang20

WORKDIR /usr/src/myapp
COPY --exclude=target . .

RUN cargo build --release --example e_d


FROM alpine:latest

RUN adduser -D worker -u 1000
USER 1000

EXPOSE 8888/TCP
EXPOSE 8080/TCP

COPY --from=builder /usr/src/myapp/target/x86_64-unknown-linux-musl/release/examples/e_d /usr/local/bin/e_d

ENTRYPOINT ["e_d"]