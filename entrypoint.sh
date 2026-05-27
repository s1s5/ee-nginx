#!/bin/sh
set -eu

if [ -n "${NGINX_CONF_FILE:-}" ]; then
    if [ "${NGINX_AUTO_RELOAD:-false}" = "true" ]; then
        # Watch the directory containing the config file for changes
        CONF_DIR="$(dirname "$NGINX_CONF_FILE")"
        /app/generator --conf-file "$NGINX_CONF_FILE" --watch "$CONF_DIR" --reload-nginx &
    else
        /app/generator --conf-file "$NGINX_CONF_FILE"
    fi
else
    /app/generator
fi

exec /docker-entrypoint.sh "$@"
