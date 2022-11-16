FROM rust:1.65 AS wasm_builder
WORKDIR /playground
RUN curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh
COPY ./tandem ./tandem
COPY ./tandem_garble_interop ./tandem_garble_interop
COPY ./tandem_http_client ./tandem_http_client
RUN wasm-pack build tandem_http_client --target web

FROM nginx
COPY --from=wasm_builder /playground/tandem_http_client/index.html /usr/share/nginx/html/index.html
COPY --from=wasm_builder /playground/tandem_http_client/pkg /usr/share/nginx/html/pkg
COPY --from=wasm_builder /playground/tandem_http_client/assets /usr/share/nginx/html/assets
EXPOSE 80
