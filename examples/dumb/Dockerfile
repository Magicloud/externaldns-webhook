FROM rust:alpine AS builder

RUN apk add --no-cache musl-dev

WORKDIR /usr/src/myapp
COPY . .

RUN cargo install --path . --example dumb


FROM alpine:latest

RUN adduser -D worker -u 1000
USER 1000

EXPOSE 8888/tcp
EXPOSE 8080/TCP

COPY --from=builder /usr/local/cargo/bin/dumb /usr/local/bin/dumb

CMD ["dumb"]