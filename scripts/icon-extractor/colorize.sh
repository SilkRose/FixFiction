#!/usr/bin/env sh

# cat colors.html | grep -oP '(?<=value=").*?(?=" )' | tr '\n' ' '
colors="dc2424 dc2488 dc24d6 a224dc 6924dc 3931cb 3159cb 318ccb 23b9cf 23cfa0 23cf68 2bcf23 60cf23 9ccf23 cfc923 ffb400 cf9423 cf5823 666666 444444 242424 d4a4e8 faba61 f9b8d2 9dd9f8 f0f2f3 fff798 fbf6fc c693c9 e8eb8f ebaf5f f0edef "

svg_files="Svg/font-awesome/* Svg/pony-emoji/*"

for color in $colors; do
	mkdir -p "Out/$color/font-awesome"
	mkdir -p "Out/$color/pony-emoji"
	for file in $svg_files; do
		dir_name=$(dirname "$file" | cut -d'/' -f2)
		file_name=$(basename "$file" | cut -d'.' -f1)
		convert "Png/$dir_name/$file_name.png" -resize "256x256>" -gravity center -background "#$color" -extent 256x256 "Out/$color/$dir_name/$file_name.png"
	done
done