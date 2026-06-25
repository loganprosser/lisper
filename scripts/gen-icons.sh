#!/usr/bin/env bash
set -euo pipefail
SRC="assets/branding/lisper-bot.svg"
OUT="src-tauri/icons"
RES="src-tauri/resources"

render() { # render <size> <dest.png>
  if command -v rsvg-convert >/dev/null; then
    rsvg-convert -w "$1" -h "$1" "$SRC" -o "$2"
  elif command -v magick >/dev/null; then
    magick -background none -size "${1}x${1}" "$SRC" "$2"
  else
    echo "Need rsvg-convert or imagemagick (magick)"; exit 1
  fi
}

# Core PNGs
render 32   "$OUT/32x32.png"
render 128  "$OUT/128x128.png"
render 256  "$OUT/128x128@2x.png"
render 512  "$OUT/icon.png"
render 512  "$OUT/logo.png"
# Windows Store / Square logos
render 30   "$OUT/Square30x30Logo.png"
render 44   "$OUT/Square44x44Logo.png"
render 71   "$OUT/Square71x71Logo.png"
render 89   "$OUT/Square89x89Logo.png"
render 107  "$OUT/Square107x107Logo.png"
render 142  "$OUT/Square142x142Logo.png"
render 150  "$OUT/Square150x150Logo.png"
render 284  "$OUT/Square284x284Logo.png"
render 310  "$OUT/Square310x310Logo.png"
render 50   "$OUT/StoreLogo.png"

# macOS .icns via iconset
if command -v iconutil >/dev/null; then
  ICONSET="$(mktemp -d)/lisper.iconset"; mkdir -p "$ICONSET"
  for s in 16 32 128 256 512; do
    render "$s"           "$ICONSET/icon_${s}x${s}.png"
    render $((s*2))       "$ICONSET/icon_${s}x${s}@2x.png"
  done
  iconutil -c icns "$ICONSET" -o "$OUT/icon.icns"
fi

# .ico (multi-size) via magick if available
if command -v magick >/dev/null; then
  magick "$OUT/256.png" 2>/dev/null || true
  magick -background none "$SRC" -define icon:auto-resize=256,128,64,48,32,16 "$OUT/icon.ico"
fi

# Tray icons (monochrome-ish, reuse mark; light + dark variants share art here)
for f in tray_idle tray_idle_dark tray_recording tray_recording_dark tray_transcribing tray_transcribing_dark; do
  render 64 "$RES/$f.png"
done
render 512 "$RES/handy.png"
render 512 "$RES/recording.png"
render 512 "$RES/transcribing.png"
echo "icons generated"
