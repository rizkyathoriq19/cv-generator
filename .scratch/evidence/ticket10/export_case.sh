#!/usr/bin/env bash
# Open a seeded CV in the packaged app (offline Ubuntu container), optionally
# choose a structure/content language, export its current PDF, and stop the app.
#
#   export_case.sh <library-title-word> <target.pdf> [structure label] [content language label]
#   e.g. export_case.sh Complete /data/ubuntu/out/complete-combination-id.pdf Combination Indonesian
#
# The library row and the select options are located by OCR: rows reorder as
# projects autosave, and a focused WebKit select leaves a popup open that blocks
# later clicks. The saved project JSON is printed so the caller can confirm the
# intended project and the persisted structure/language.
set -euo pipefail

TITLE="$1"
TARGET="$2"
STRUCT="${3:-}"
LANGD="${4:-}"

DATA=/home/airzy/.hermes/cache/scratch/cvgen-ticket10/ubuntu
SHOTS="$DATA/shots"
DRIVE=/work/.scratch/evidence/ticket10/drive.py

d() {
  docker exec -e CVGEN_BIN=/usr/bin/cv-generator -e CVGEN_DATA=/data/ubuntu \
    -e CVGEN_SHOTS=/data/ubuntu/shots -e WEBKIT_DISABLE_SANDBOX_THIS_IS_DANGEROUS=1 \
    cvgen-u2404-run python3 "$DRIVE" "$@"
}

# Screenshots are cropped to the 1240x900 window and doubled before OCR; halve
# the boxes back to screen coordinates.
word_xy() {
  magick "$SHOTS/$1.png" -crop 1240x900+0+0 +repage -resize 200% "$SHOTS/$1-ocr.png"
  tesseract "$SHOTS/$1-ocr.png" - tsv 2>/dev/null |
    awk -F'\t' -v w="$2" 'NR>1 && $12==w {print int($7/2 + $9/4), int($8/2 + $10/4); exit}'
}

row_y() {
  d shot "$1" >/dev/null
  magick "$SHOTS/$1.png" -crop 1240x900+0+0 +repage -resize 200% "$SHOTS/$1-rows.png"
  tesseract "$SHOTS/$1-rows.png" - tsv 2>/dev/null |
    awk -F'\t' -v t="$TITLE" 'NR>1 && $12==t {print int($8/2); exit}'
}

select_option() {
  local x="$1" y="$2" label="$3" shot="$4" xy
  d click "$x" "$y"
  sleep 1
  d shot "$shot"
  xy=$(word_xy "$shot" "$label")
  [ -n "$xy" ] || { echo "option '$label' not found in OCR" >&2; exit 1; }
  echo "option '$label' at $xy"
  d click $xy
  sleep 2
}

d stop
d start >/dev/null
# ponytail: fixed window-geometry coordinates (1240-wide, 2100-tall run). Re-derive
# them from a screenshot if the editor layout changes.
y=$(row_y open-library)
[ -n "$y" ] || { echo "title '$TITLE' not found in library OCR" >&2; exit 1; }
echo "row '$TITLE' at y=$y"
d click 196 $((y + 8))
sleep 2
d resize 1240 2100 >/dev/null

[ -n "$STRUCT" ] && select_option 205 188 "$STRUCT" select-structure
[ -n "$LANGD" ] && select_option 205 288 "$LANGD" select-language

d wheel 200 1200 200
d shot "$(basename "$TARGET" .pdf)-bottom"
rm -f "$DATA/out/$(basename "$TARGET")"
d click 189 1871
sleep 2
d click 175 18
sleep 0.4
d ctrl a
d type "$TARGET"
sleep 1
d enter
sleep 3
d stop

echo "--- files"
ls -la --time-style=+%H:%M:%S "$DATA/out/"
echo "--- project state"
for f in "$DATA"/data/app.cvgen.desktop/projects/t10*.json; do
  python3 - "$f" <<'PY'
import json,sys,pathlib
p=pathlib.Path(sys.argv[1])
try:
    d=json.loads(p.read_text())
except Exception as exc:
    print(f"{p.name} UNREADABLE ({exc})")
else:
    print(f"{p.name} structure={d['structure']} content={d['content_language']} name={d['cv']['name']!r}")
PY
done
echo "--- pdf first lines"
pdftotext -f 1 -l 1 "$DATA/out/$(basename "$TARGET")" - 2>/dev/null | sed -n '1,8p'
