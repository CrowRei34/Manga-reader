#!/usr/bin/env bash
set -euo pipefail

readonly REPOSITORY="https://github.com/AppFuton/futon-parsers.git"
readonly BUILD_FILE="${1:-daemon/build.gradle.kts}"

if [[ ! -f "$BUILD_FILE" ]]; then
  echo "No se encontró $BUILD_FILE" >&2
  exit 1
fi

latest_sha="$(git ls-remote "$REPOSITORY" refs/heads/master | awk '{print $1}')"
current_sha="$(sed -nE 's/.*futon-parsers:([0-9a-f]{40}).*/\1/p' "$BUILD_FILE")"

if [[ ! "$latest_sha" =~ ^[0-9a-f]{40}$ ]]; then
  echo "No se pudo resolver el HEAD de Futon Parsers" >&2
  exit 1
fi

if [[ ! "$current_sha" =~ ^[0-9a-f]{40}$ ]]; then
  echo "No se pudo identificar la revisión actual en $BUILD_FILE" >&2
  exit 1
fi

echo "current_sha=$current_sha"
echo "latest_sha=$latest_sha"

if [[ "$current_sha" == "$latest_sha" ]]; then
  echo "changed=false"
  exit 0
fi

sed -i "s/futon-parsers:$current_sha/futon-parsers:$latest_sha/" "$BUILD_FILE"
echo "changed=true"

