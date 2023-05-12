#!/bin/bash
# -*- mode: shell-script -*-

set -eu  # <= 0以外が返るものがあったら止まる, 未定義の変数を使おうとしたときに打ち止め

CONF_PATH=`pwd`/conf
CONTAINER_NAME=nginx-`date "+%Y%m%d%H%M%S"`

docker run --rm -v `pwd`/mnt:/mnt -v ${CONF_PATH}:/conf -e NGINX_CONF_FILE=/conf -e RUST_LOG=debug --name ${CONTAINER_NAME} --network host s1s5/ee-nginx &

function trap_bg () {
    docker stop ${CONTAINER_NAME}
}

trap trap_bg INT
trap trap_bg ERR



inotifywait -m -e modify  "$CONF_PATH" |
    while read -r line; do
        if [[ $line == *"$CONF_PATH"* ]]; then
            docker exec ${CONTAINER_NAME} /bin/sh -c 'rm -rf /etc/nginx/conf.d/; mkdir /etc/nginx/conf.d/'
            docker exec ${CONTAINER_NAME} /app/generator --conf-file /conf
            docker exec ${CONTAINER_NAME} nginx -s reload
        fi
    done
