#!/usr/bin/env bash
set -euo pipefail

REPO="CrowRei34/Manga-reader"
ASSET="Bakeneko-Portable-Linux-x86_64.tar.gz"
BASE="${XDG_DATA_HOME:-$HOME/.local/share}/bakeneko"
RELEASES="$BASE/releases"
BIN_DIR="${XDG_BIN_HOME:-$HOME/.local/bin}"
DESKTOP_DIR="${XDG_DATA_HOME:-$HOME/.local/share}/applications"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/bakeneko-install.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

command -v curl >/dev/null || { echo "ERROR: curl es necesario." >&2; exit 1; }
command -v tar >/dev/null || { echo "ERROR: tar es necesario." >&2; exit 1; }

URL="https://github.com/$REPO/releases/latest/download/$ASSET"
CHECKSUM_URL="$URL.sha256"
ARCHIVE="$TMP_DIR/$ASSET"
echo "Descargando Bakeneko desde GitHub…"
curl --fail --location --retry 3 --proto '=https' --tlsv1.2 -o "$ARCHIVE" "$URL"
curl --fail --location --retry 3 --proto '=https' --tlsv1.2 -o "$TMP_DIR/$ASSET.sha256" "$CHECKSUM_URL"
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
