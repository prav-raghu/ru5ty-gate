#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")/../.."

if [ -z "${DATABASE_URL:-}" ]; then
  echo "ERROR: DATABASE_URL is not set." >&2
  exit 1
fi

cargo run --release --bin admin-api -- migrate

echo "Database migrations completed successfully."
