# syntax=docker/dockerfile:1.7-labs

FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /usr/src/myapp
COPY --exclude=./target/ . .

RUN cargo install --path .


FROM alpine:latest

RUN adduser -D worker -u 1000
USER 1000

EXPOSE 8888/TCP
EXPOSE 8080/TCP

COPY --from=builder /usr/local/cargo/bin/e_d /usr/local/bin/e_d

ENTRYPOINT ["e_d"]