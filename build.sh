#!/bin/bash
# -*- mode: shell-script -*-

set -eu  # <= 0以外が返るものがあったら止まる, 未定義の変数を使おうとしたときに打ち止め


docker buildx build --platform linux/amd64,linux/aarch64 -f multi-platform.Dockerfile --push -t s1s5/ee-nginx .
