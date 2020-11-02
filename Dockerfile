FROM rust:alpine AS builder
WORKDIR /build/
COPY . /build/
RUN apk add libc-dev openssl-dev
RUN cargo build --release

FROM alpine:latest
WORKDIR /root/
COPY --from=builder /build/target/release/keymaker .
CMD ["./keymaker"]
EXPOSE 80
