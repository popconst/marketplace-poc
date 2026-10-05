#!/bin/sh
# Builds and starts the marketplace with demo data, using whichever Docker Compose is installed.
# Extra arguments go to `up`, e.g. `./start.sh -d` to run in the background.
set -e
cd "$(dirname "$0")"

# Saves a host port in .env on the first run: the default, or the next one up if something on
# this machine already listens there. Later runs keep it, and a port set in the environment wins.
pick_port() { # name, default, value from the environment
    [ -n "$3" ] && return
    grep -qs "^$1=" .env && return
    port=$2
    while nc -z 127.0.0.1 "$port" 2>/dev/null; do port=$((port + 1)); done
    echo "$1=$port" >> .env
}
pick_port WEB_HOST_PORT 8080 "$WEB_HOST_PORT"
pick_port POSTGRES_HOST_PORT 5433 "$POSTGRES_HOST_PORT"
web_port=${WEB_HOST_PORT:-$(sed -n 's/^WEB_HOST_PORT=//p' .env | tail -n 1)}
echo "The app will be at http://localhost:$web_port once the seed service has finished."

if docker compose version >/dev/null 2>&1; then
    exec docker compose --profile seed up --build "$@"
elif command -v docker-compose >/dev/null 2>&1; then
    exec docker-compose --profile seed up --build "$@"
else
    echo "Docker with Compose is needed: https://docs.docker.com/get-docker/" >&2
    exit 1
fi
