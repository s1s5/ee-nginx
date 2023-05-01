# つくるアプリによってここの名称を変更すること
ARG APP_NAME="ee-nginx"

# ------------- build ----------------
# FROM ekidd/rust-musl-builder:stable as builder
FROM clux/muslrust:1.67.1 as builder

RUN mkdir -p /home/rust/src
WORKDIR /home/rust

ARG APP_NAME

COPY Cargo.toml Cargo.lock ./
# 適当な実行ファイルの生成
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > /home/rust/src/main.rs
# 依存関係のみ先にコンパイルして、キャッシュしておく
RUN cargo build --release

# ここでちゃんとけせてないと正しくバイナリが生成されない
RUN rm target/x86_64-unknown-linux-musl/release/deps/`echo ${APP_NAME} | sed 's/-/_/'`-* target/x86_64-unknown-linux-musl/release/${APP_NAME}
RUN rm src/main.rs

# ちゃんと下バイナリを再生成
COPY ./src/ ./src/
COPY ./templates/ ./templates/
RUN cargo build --release --bin ${APP_NAME}

# ------------- runtime ----------------
FROM nginx:1.23.4-alpine

ARG APP_NAME

WORKDIR /app
COPY --from=builder /home/rust/target/x86_64-unknown-linux-musl/release/$APP_NAME ./generator

ENV RUST_LOG info
ENV NGINX_CONF "/>/usr/share/nginx/html/"

WORKDIR /
RUN echo $'#!bin/sh\n\
    /app/generator\n\
    /docker-entrypoint.sh "$@"' > /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT [ "/entrypoint.sh" ]
CMD ["nginx", "-g", "daemon off;"]
