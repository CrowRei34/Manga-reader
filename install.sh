#!/usr/bin/env bash
set -euo pipefail

REPO="CrowRei34/Manga-reader"
BASE="${XDG_DATA_HOME:-$HOME/.local/share}/bakeneko"
RELEASES="$BASE/releases"
BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DESKTOP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bakeneko-install.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

if [ "${1:-}" = "--uninstall" ] || [ "${1:-}" = "uninstall" ]; then
  echo "Desinstalando Bakeneko (se conservan biblioteca y configuración)…"
  if [ -L "$BIN_DIR/bakeneko" ]; then rm -f "$BIN_DIR/bakeneko"; fi
  rm -f "$DESKTOP_DIR/bakeneko.desktop"
  # BASE solo contiene versiones del programa; XDG_CONFIG_HOME y la base de
  # datos de la biblioteca viven fuera y no se tocan.
  if [ -d "$BASE" ]; then rm -rf "$BASE"; fi
  echo "Bakeneko desinstalado. La biblioteca permanece en tus datos locales."
  exit 0
fi

if [ "${1:-}" != "" ] && [ "${1:-}" != "--update" ] && [ "${1:-}" != "update" ]; then
  echo "Uso: $0 [--update|--uninstall]" >&2
  exit 2
fi

command -v curl >/dev/null || { echo "ERROR: curl es necesario." >&2; exit 1; }
command -v tar >/dev/null || { echo "ERROR: tar es necesario." >&2; exit 1; }

RELEASE_JSON="$(curl --fail --location --retry 3 --proto '=https' --tlsv1.2 \
  "https://api.github.com/repos/$REPO/releases/latest")"
TAG="$(printf '%s' "$RELEASE_JSON" | sed -n 's/.*"tag_name": "\([^"]*\)".*/\1/p' | head -n1)"
if [ -z "$TAG" ]; then
  echo "ERROR: GitHub no tiene un release publicado todavía." >&2
  exit 1
fi
VERSION="${TAG#v}"
ASSET="Bakeneko-Portable-v${VERSION}-Linux-x86_64.tar.gz"
URL="https://github.com/$REPO/releases/download/$TAG/$ASSET"
CHECKSUM_URL="https://github.com/$REPO/releases/download/$TAG/SHA256SUMS"
ARCHIVE="$TMP_DIR/$ASSET"
echo "Descargando Bakeneko desde GitHub…"
curl --fail --location --retry 3 --proto '=https' --tlsv1.2 -o "$ARCHIVE" "$URL"
curl --fail --location --retry 3 --proto '=https' --tlsv1.2 -o "$TMP_DIR/SHA256SUMS" "$CHECKSUM_URL"
grep "  $ASSET$" "$TMP_DIR/SHA256SUMS" > "$TMP_DIR/$ASSET.sha256"
(cd "$TMP_DIR" && sha256sum -c "$ASSET.sha256")

tar -xzf "$ARCHIVE" -C "$TMP_DIR"
TOP="$(find "$TMP_DIR" -mindepth 1 -maxdepth 1 -type d -name 'Bakeneko-Portable-*' -print -quit)"
if [ -z "$TOP" ] || [ ! -x "$TOP/bakeneko" ]; then
  echo "ERROR: el paquete no tiene una estructura válida." >&2
  exit 1
fi
VERSION="$(basename "$TOP" | sed 's/^Bakeneko-Portable-v//; s/-Linux-x86_64$//')"
TARGET="$RELEASES/$VERSION"
mkdir -p "$RELEASES" "$BIN_DIR" "$DESKTOP_DIR"
if [ ! -e "$TARGET" ]; then
  mv "$TOP" "$TARGET"
fi
ln -sfn "$TARGET" "$BASE/current"
ln -sfn "$BASE/current/bakeneko" "$BIN_DIR/bakeneko"
cat > "$DESKTOP_DIR/bakeneko.desktop" <<EOF
[Desktop Entry]
Name=Bakeneko Reader
Comment=Lector de manga
Exec=$BIN_DIR/bakeneko
Icon=$BASE/current/app/assets/bakeneko.png
Terminal=false
Type=Application
Categories=Utility;Graphics;
EOF
echo "Bakeneko $VERSION instalado en $TARGET"
echo "Ejecuta: $BIN_DIR/bakeneko"
case ":${PATH:-}:" in
  *:"$BIN_DIR":*) ;;
  *) echo "Añade $BIN_DIR a PATH si el comando no se encuentra." ;;
esac
