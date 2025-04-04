#!/usr/bin/env sh

echo "Extracting pony emojis..."
./extract-pony-emoji.sh || {
	echo "Failed to extract pony emojis."
	exit 1
}
echo "Extracting font-awesome icons..."
./extract-font-awesome.sh || {
	echo "Failed to extract font awesome icons."
	exit 1
}
echo "Converting icons to PNG..."
./convert.sh || {
	echo "Failed to convert icons."
	exit 1
}
echo "Colorizing icons..."
./colorize.sh || {
	echo "Failed to Colorize icons."
	exit 1
}