#!/bin/sh
# Installs an AppImage for this user, in a location that the updater can replace.
set -eu
if [ "$#" -ne 2 ]; then echo "Usage: install-appimage.sh path/to/App.AppImage product-slug" >&2; exit 2; fi
ULIX_IMAGE=$1
ULIX_PRODUCT=$2
case "$ULIX_PRODUCT" in ''|*[!a-z0-9-]*) echo 'Invalid product slug' >&2; exit 2;; esac
ULIX_DEST="$HOME/Applications/$ULIX_PRODUCT.AppImage"
mkdir -p "$HOME/Applications" "$HOME/.local/share/applications"
cp "$ULIX_IMAGE" "$ULIX_DEST.new"
chmod 755 "$ULIX_DEST.new"
mv "$ULIX_DEST.new" "$ULIX_DEST"
# Desktop Exec arguments are quoted; escape reserved quoted-argument characters.
ULIX_ESCAPED=$(printf '%s' "$ULIX_DEST" | sed 's/[\\"`$]/\\&/g')
cat > "$HOME/.local/share/applications/ai.ulix.$ULIX_PRODUCT.desktop" <<DESKTOP
[Desktop Entry]
Type=Application
Name=$ULIX_PRODUCT
Exec="$ULIX_ESCAPED"
Terminal=false
Categories=Utility;
DESKTOP
printf 'Installed %s\n' "$ULIX_DEST"
