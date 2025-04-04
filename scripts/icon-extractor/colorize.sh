#!/usr/bin/env sh

# cat colors.html | grep -oP '(?<=value=").*?(?=" )' | tr '\n' ' '
colors="dc2424 dc2488 dc24d6 a224dc 6924dc 3931cb 3159cb 318ccb 23b9cf 23cfa0 23cf68 2bcf23 60cf23 9ccf23 cfc923 ffb400 cf9423 cf5823 666666 444444 242424 d4a4e8 faba61 f9b8d2 9dd9f8 f0f2f3 fff798 fbf6fc c693c9 e8eb8f ebaf5f f0edef "

font_awesome_files="Svg/font-awesome/*"
pony_emoji_files="Svg/pony-emoji/*"

out_dir="Out/"
mkdir -p "$out_dir"
for color in $colors; do
	mkdir -p "$out_dir/$color/"
	for file in $font_awesome_files; do
		filename=$(basename "$file")
		mkdir -p "$out_dir/$color/font-awesome/"
		sed "s/#fff/#$color/g" $file > "$out_dir/$color/font-awesome/$filename"
	done
	for file in $pony_emoji_files; do
		filename=$(basename "$file")
		mkdir -p "$out_dir/$color/pony-emoji/"
		sed "s/currentColor/#$color/g" $file > "$out_dir/$color/pony-emoji/$filename"
	done
done