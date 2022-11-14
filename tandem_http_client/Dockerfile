FROM rust:latest
WORKDIR /notebook
COPY . .
RUN cargo run --features="bin"
RUN wasm-pack build --target web

FROM nginx
COPY . /usr/share/nginx/html
EXPOSE 80
