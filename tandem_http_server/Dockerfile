FROM rust:latest

WORKDIR /wrk17-alpha
COPY ./m1 ./m1
COPY ./m1_garble_interop ./m1_garble_interop
COPY ./m1_http_server ./m1_http_server

RUN cargo install --features="bin" --path ./m1_http_server

ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_PORT=8000
EXPOSE 8000
CMD ["m1_http_server"]
