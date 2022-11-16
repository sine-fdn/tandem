FROM rust:1.65 AS builder
WORKDIR /usr/src/tandem_http_server
COPY ./tandem ./tandem
COPY ./tandem_garble_interop ./tandem_garble_interop
COPY ./tandem_http_server ./tandem_http_server
RUN rustup target add x86_64-unknown-linux-musl
RUN cargo install --target x86_64-unknown-linux-musl --features="bin" --path ./tandem_http_server

FROM alpine:3.16
COPY --from=builder /usr/local/cargo/bin/tandem_http_server /usr/local/bin/tandem_http_server

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000
EXPOSE 8000
CMD ["tandem_http_server"]
