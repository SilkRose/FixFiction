#!/usr/bin/env sh

# cat colors.html | grep -oP '(?<=value=").*?(?=" )' | tr '\n' ' '
colors="dc2424 dc2488 dc24d6 a224dc 6924dc 3931cb 3159cb 318ccb 23b9cf 23cfa0 23cf68 2bcf23 60cf23 9ccf23 cfc923 ffb400 cf9423 cf5823 666666 444444 242424 d4a4e8 faba61 f9b8d2 9dd9f8 f0f2f3 fff798 fbf6fc c693c9 e8eb8f ebaf5f f0edef "

svg_files="Svg/font-awesome/* Svg/pony-emoji/*"
out_dir="Out/"

for color in $colors; do
	mkdir -p "$out_dir/$color/font-awesome"
	mkdir -p "$out_dir/$color/pony-emoji"
	for file in $svg_files; do
		dir_name=$(dirname "$file" | cut -d'/' -f2)
		file_name=$(basename "$file" | cut -d'.' -f1)
		out_filename="$out_dir/$color/$dir_name/$file_name.png"

		svg_props=$(inkscape "$file" --query-all)
		svg_width=$(echo "$svg_props" | head -n 1 | cut -d',' -f4)
		svg_height=$(echo "$svg_props" | head -n 1 | cut -d',' -f5)

		if [ `echo "$svg_width > $svg_height" | bc` = "1" ]; then
			export_width="256"
			export_height=$(echo "scale=4;($svg_height / $svg_width) * 256" | bc | cut -d"." -f1)
		else
			export_height="256"
			export_width=$(echo "scale=4;($svg_width / $svg_height) * 256" | bc | cut -d"." -f1)
		fi

		inkscape "$file" --export-area-drawing --export-type="png" --export-width="$export_width" --export-height="$export_height" --export-filename="$out_filename" --export-background="#$color" --export-background-opacity="1"
		convert "$out_filename" -resize "256x256>" -gravity center -background "#$color" -extent 256x256 "$out_filename" 
	done
done