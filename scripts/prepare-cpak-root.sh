#!/usr/bin/env bash
set -euo pipefail

if [[ $# -ne 1 ]]; then
  echo "Uso: $0 VERSION" >&2
  exit 2
fi

version="$1"
package="dist/Bakeneko-Portable-v${version}-Linux-x86_64.tar.gz"
root="dist/cpak-root"

if [[ ! -f "$package" ]]; then
  echo "Falta el paquete portátil: $package" >&2
  exit 1
fi

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

rm -rf "$root"
mkdir -p "$root/opt/bakeneko"
tar --extract --gzip --file "$package" \
  --directory "$tmp" \
  --strip-components=1 \
  --no-same-owner \
  --no-same-permissions
cp -a "$tmp/." "$root/opt/bakeneko/"

install -Dm755 packaging/bakeneko-cpak \
  "$root/usr/bin/bakeneko"
install -Dm644 packaging/bakeneko.desktop \
  "$root/usr/share/applications/com.crowrei34.Bakeneko.desktop"
install -Dm644 assets/bakeneko.svg \
  "$root/usr/share/icons/hicolor/scalable/apps/bakeneko.svg"

test -x "$root/usr/bin/bakeneko"
test -x "$root/opt/bakeneko/app/bakeneko"
test -x "$root/opt/bakeneko/app/jre/bin/java"
test -f "$root/usr/share/applications/com.crowrei34.Bakeneko.desktop"
test -f "$root/usr/share/icons/hicolor/scalable/apps/bakeneko.svg"

echo "Raíz cpak preparada en $root"
