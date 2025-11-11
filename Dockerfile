FROM rustlang/rust:nightly-alpine AS builder

RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static

WORKDIR /usr/src/app
COPY . .

RUN rustup target add x86_64-unknown-linux-musl \
 && cargo build --release --target x86_64-unknown-linux-musl \
 && strip target/x86_64-unknown-linux-musl/release/crunchyma

FROM alpine:latest

COPY --from=builder /usr/src/app/target/x86_64-unknown-linux-musl/release/crunchyma /usr/local/bin/crunchyma

ENTRYPOINT ["/usr/local/bin/crunchyma"]