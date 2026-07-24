#!/usr/bin/env bash

set -euo pipefail

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
project_dir="$(cd -- "$script_dir/.." && pwd)"
builder_image="powerpico-client-linux-builder:ubuntu22.04"
builder_container=""

cleanup() {
  if [[ -n "$builder_container" ]]; then
    docker rm -f "$builder_container" >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

cd "$project_dir"
mkdir -p artifacts/linux

docker build \
  --file packaging/linux/Dockerfile \
  --target artifacts \
  --tag "$builder_image" \
  .

builder_container="$(docker create "$builder_image" /artifact-export)"
docker cp "$builder_container:/deb/." artifacts/linux/
docker cp "$builder_container:/appimage/." artifacts/linux/

echo "Linux packages written to $project_dir/artifacts/linux"
