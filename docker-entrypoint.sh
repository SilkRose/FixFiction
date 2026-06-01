#!/bin/sh
set -eu

if [ "$(id -u)" = "0" ]; then
	mkdir -p /app/logs
	chmod 0777 /app/logs || true
	exec runuser -u app -- "$@"
fi

exec "$@"
