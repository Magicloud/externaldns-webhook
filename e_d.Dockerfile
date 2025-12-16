# syntax=docker/dockerfile:1.7-labs

FROM ghcr.io/magicloud/rust-stable:latest AS builder

RUN apk add --no-cache musl-dev g++ clang20

WORKDIR /usr/src/myapp
COPY --exclude=target . .

RUN cargo build --release --example e_d


FROM alpine:latest

RUN adduser -D worker -u 1000
USER 1000

EXPOSE 8888/TCP
EXPOSE 8080/TCP

COPY --from=builder /usr/src/myapp/target/release/examples/e_d /usr/local/bin/e_d

ENTRYPOINT ["e_d"]