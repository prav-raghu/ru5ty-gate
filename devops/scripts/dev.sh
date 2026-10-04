#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/../.."

cargo build --workspace --bins

pids=()
trap 'kill "${pids[@]}" 2>/dev/null || true' EXIT INT TERM

for service in admin-api customer-api schedule-api api-gateway; do
  (cd "apps/backend/$service" && set -a && [ -f .env ] && . ./.env; set +a; exec cargo run --quiet --bin "$service") &
  pids+=("$!")
done

wait
