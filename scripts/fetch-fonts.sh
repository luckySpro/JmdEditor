#!/usr/bin/env bash
# Downloads Noto Sans/Serif CJK (SIL Open Font License) for PDF export, pinned
# to tagged releases and checked against SHA-256 sums.
#
#   scripts/fetch-fonts.sh [target-dir]     default: apps/desktop/src-tauri/resources/fonts
#
# The desktop build bundles that folder; jmd-server reads fonts from JMD_FONT_DIRS.
set -euo pipefail

dest="${1:-$(dirname "$0")/../apps/desktop/src-tauri/resources/fonts}"
base="https://raw.githubusercontent.com/notofonts/noto-cjk"
mkdir -p "$dest"

# tag/path  sha256
fonts=(
  "Sans2.004/Sans/SubsetOTF/SC/NotoSansSC-Regular.otf faa6c9df652116dde789d351359f3d7e5d2285a2b2a1f04a2d7244df706d5ea9"
  "Sans2.004/Sans/SubsetOTF/SC/NotoSansSC-Bold.otf c6cb5a93abaa9edc8ee7463b7ebb7f42d618d40e6ed2f7a5371c97b0b64767c0"
  "Serif2.003/Serif/SubsetOTF/SC/NotoSerifSC-Regular.otf e8f396decc1f0963a016a989c3d8852e863d1350996f573860a80767c83a1cd3"
  "Serif2.003/Serif/SubsetOTF/SC/NotoSerifSC-Bold.otf 24693d48bdb9152f0a06b02af625638a1097abd6de4010ebba027f6e82710527"
  "Sans2.004/Sans/SubsetOTF/TC/NotoSansTC-Regular.otf 5bab0cb3c1cf89dde07c4a95a4054b195afbcfe784d69d75c340780712237537"
  "Sans2.004/Sans/SubsetOTF/JP/NotoSansJP-Regular.otf dff723ba59d57d136764a04b9b2d03205544f7cd785a711442d6d2d085ac5073"
)

sha() {
  if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi
}

for entry in "${fonts[@]}"; do
  path="${entry% *}"
  want="${entry##* }"
  file="$dest/$(basename "$path")"
  if [[ -f "$file" && "$(sha "$file")" == "$want" ]]; then
    echo "ok       $(basename "$file")"
    continue
  fi
  echo "download $(basename "$file")"
  curl -fsSL --retry 3 -o "$file.part" "$base/$path"
  got="$(sha "$file.part")"
  if [[ "$got" != "$want" ]]; then
    rm -f "$file.part"
    echo "checksum mismatch for $path (got $got)" >&2
    exit 1
  fi
  mv "$file.part" "$file"
done
echo "fonts ready in $dest"
