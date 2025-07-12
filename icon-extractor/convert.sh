#!/usr/bin/env sh

svg_files="Svg/font-awesome/* Svg/pony-emoji/*"

mkdir -p "Png/font-awesome"
mkdir -p "Png/pony-emoji"
for file in $svg_files; do
	dir_name=$(dirname "$file" | cut -d'/' -f2)
	file_name=$(basename "$file" | cut -d'.' -f1)
	out_filename=""

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

	inkscape "$file" --export-area-drawing --export-filename="Png/$dir_name/$file_name.png" --export-type="png" --export-width="$export_width" --export-height="$export_height"
done