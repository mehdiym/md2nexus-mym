#!/usr/bin/env bash
# Draw the README's preview of each color theme, docs/themes/<theme>.png: a heading in the main
# color, a smaller heading and a table header row in the soft one, on a dark background close to
# the Nexus's. The colors are read from the Theme enum of src/main.rs, so rerun this script after
# changing a palette. Needs ImageMagick 7 and the Noto Sans fonts.
set -euo pipefail
cd "$(dirname "$0")/.."

fonts=/usr/share/fonts/truetype/noto
bold=$fonts/NotoSans-Bold.ttf
semibold=$fonts/NotoSans-SemiBold.ttf
regular=$fonts/NotoSans-Regular.ttf
background='#222326'
text='#d4d4d4'
grid='#4a4c50'

mkdir -p docs/themes

draw() {
    local name=$1 main=$2 soft=$3
    magick -size 280x112 "xc:$background" \
        -fill "$main" -font "$bold" -pointsize 20 -annotate +12+30 'Heading' \
        -fill "$soft" -font "$semibold" -pointsize 15 -annotate +12+55 'Smaller heading' \
        -fill none -stroke "$grid" -strokewidth 1 \
        -draw 'rectangle 12,66 267,104' -draw 'line 12,85 267,85' -draw 'line 140,66 140,104' \
        -stroke none \
        -fill "$soft" -font "$bold" -pointsize 12 \
        -annotate +20+80 'Header' -annotate +148+80 'Header' \
        -fill "$text" -font "$regular" -pointsize 12 \
        -annotate +20+99 'Text' -annotate +148+99 'Text' \
        -strip "docs/themes/$name.png"
}

# The theme without colors: everything in the text color.
draw none "$text" "$text"

# Theme::Gold => Some(("#caa560", "#dfc99f")),
sed -nE 's/.*Theme::([A-Za-z]+) => Some\(\("(#[0-9a-fA-F]{6})", "(#[0-9a-fA-F]{6})"\)\).*/\1 \2 \3/p' \
    src/main.rs |
    while read -r name main soft; do
        draw "${name,,}" "$main" "$soft"
    done

ls docs/themes
