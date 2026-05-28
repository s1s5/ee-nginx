# つくるアプリによってここの名称を変更すること
ARG APP_NAME="ee-nginx"

# ------------- build ----------------
FROM --platform=$BUILDPLATFORM ghcr.io/rust-cross/rust-musl-cross:${TARGETARCH}-musl AS builder

RUN mkdir -p /home/rust/src
WORKDIR /home/rust

ARG APP_NAME

COPY Cargo.toml Cargo.lock ./
# 適当な実行ファイルの生成
RUN echo "fn main() {println!(\"if you see this, the build broke\")}" > /home/rust/src/main.rs
# 依存関係のみ先にコンパイルして、キャッシュしておく
RUN cargo build --release

# ここでちゃんとけせてないと正しくバイナリが生成されない
RUN rm target/*-unknown-linux-musl/release/deps/`echo ${APP_NAME} | sed 's/-/_/'`-* target/*-unknown-linux-musl/release/${APP_NAME}
RUN rm src/main.rs

# ちゃんと下バイナリを再生成
COPY ./src/ ./src/
COPY ./templates/ ./templates/
RUN cargo build --release --bin ${APP_NAME}

# ------------- runtime ----------------
FROM nginx:stable-alpine

ARG APP_NAME

WORKDIR /app
COPY --from=builder /home/rust/target/*-unknown-linux-musl/release/$APP_NAME ./generator

ENV RUST_LOG=info \
    NGINX_CONF="/>/usr/share/nginx/html/" \
    NGINX_CONF_FILE="" \
    NGINX_IN_DOCKER="true"

WORKDIR /
COPY entrypoint.sh /entrypoint.sh
RUN chmod +x /entrypoint.sh

ENTRYPOINT [ "/entrypoint.sh" ]
CMD ["nginx", "-g", "daemon off;"]
