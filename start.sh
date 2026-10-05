#!/bin/sh
# Builds and starts the marketplace with demo data, using whichever Docker Compose is installed.
# Extra arguments go to `up`, e.g. `./start.sh -d` to run in the background.
set -e
cd "$(dirname "$0")"
if docker compose version >/dev/null 2>&1; then
    exec docker compose --profile seed up --build "$@"
elif command -v docker-compose >/dev/null 2>&1; then
    exec docker-compose --profile seed up --build "$@"
else
    echo "Docker with Compose is needed: https://docs.docker.com/get-docker/" >&2
    exit 1
fi
