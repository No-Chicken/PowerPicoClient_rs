#!/usr/bin/env bash
# Build the Linux and Windows packages locally and upload them to the GitHub
# release for the given tag. CI builds and publishes the macOS package; this
# script adds the remaining assets and refreshes SHA256SUMS over everything.
#
# Usage: scripts/upload-release.sh v0.4.0
#
# Requirements: gh (authenticated), rustup target x86_64-pc-windows-msvc,
# cargo-xwin, clang + lld, zip. Run it after the tag's CI run has published
# the release so the notes file is applied.
set -euo pipefail

tag="${1:?usage: scripts/upload-release.sh vX.Y.Z}"
case "$tag" in
  v*) ;;
  *) tag="v$tag" ;;
esac
tag="${tag%%+*}" # strip any build metadata

cd "$(dirname "$0")/.."

# The packaged version must match the tag, same rule as CI.
version=$(cargo metadata --no-deps --format-version 1 \
  | jq -r '.packages[] | select(.name == "powerpico-app") | .version')
if [ "v${version}" != "${tag}" ]; then
  echo "tag ${tag} does not match powerpico-app ${version}" >&2
  exit 1
fi

if ! gh release view "$tag" >/dev/null 2>&1; then
  echo "release ${tag} does not exist yet; wait for the tag's CI run to publish it" >&2
  exit 1
fi

# ------------------------------------------------------------- Linux x86_64
echo "== Building Linux release =="
S=~/.workbuddy/skills/adaptive-rust-build/scripts
if [ -x "$S/adaptive-cargo" ]; then
  "$S/adaptive-cargo" --mode ecore --nice 10 build --release -p powerpico-app
else
  cargo build --release -p powerpico-app
fi

linux_name="powerpico-client-${tag}-linux-x86_64"
mkdir -p "dist/${linux_name}/docs"
install -m 755 target/release/powerpico-client "dist/${linux_name}/powerpico-client"
install -m 644 LICENSE README.md THIRD_PARTY_NOTICES.md "dist/${linux_name}/"
cp -r docs/*.md "dist/${linux_name}/docs/"
tar -C dist -czf "dist/${linux_name}.tar.gz" "${linux_name}"

# ----------------------------------------------------- Windows x86_64 (cross)
echo "== Building Windows release (cross) =="
rustup target add x86_64-pc-windows-msvc
# +crt-static keeps the exe free of the VC++ redistributable.
RUSTFLAGS="-C target-feature=+crt-static" \
  cargo xwin build --release --target x86_64-pc-windows-msvc -p powerpico-app

win_name="powerpico-client-${tag}-windows-x86_64"
mkdir -p "dist/${win_name}/docs"
install -m 755 \
  target/x86_64-pc-windows-msvc/release/powerpico-client.exe \
  "dist/${win_name}/powerpico-client.exe"
install -m 644 LICENSE README.md THIRD_PARTY_NOTICES.md "dist/${win_name}/"
cp -r docs/*.md "dist/${win_name}/docs/"
(cd dist && zip -qr "${win_name}.zip" "${win_name}")

# ------------------------------------------------------------------- upload
echo "== Uploading to ${tag} =="
# Fetch whatever CI already published so SHA256SUMS covers every asset.
fetch="dist/fetch"
rm -rf "$fetch"
mkdir -p "$fetch"
gh release download "$tag" -D "$fetch"

(cd "$fetch" && sha256sum ./* > SHA256SUMS)

gh release upload "$tag" \
  "dist/${linux_name}.tar.gz" \
  "dist/${win_name}.zip" \
  "$fetch/SHA256SUMS" \
  --clobber

echo "== Done: ${tag} assets =="
gh release view "$tag" --json assets --jq '.assets[].name'
