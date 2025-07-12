#!/usr/bin/env sh

if [ ! -f FontAwesome.tar.gz ]; then
	wget "https://github.com/Rush/Font-Awesome-SVG-PNG/archive/refs/tags/1.1.5.tar.gz" -O FontAwesome.tar.gz || {
		echo "Failed to download FontAwesome."
		exit 1
	}
fi
if [ ! -d FontAwesome/ ]; then
	mkdir -p FontAwesome/
	tar -xvzf FontAwesome.tar.gz --strip-components=1 -C FontAwesome/ || {
		echo "Failed to extract FontAwesome."
		exit 1
	}
fi

# cat icons.html | grep -oP '(?<=data-icon-type="font-awesome" class="bookshelf-icon-element fa fa-).*?(?="></span>)' | tr '\n' ' '
icons="bed book bookmark bolt certificate check clock-o cog exclamation eye film folder-open frown-o gamepad globe heart line-chart lock magic meh-o minus-square music paw pencil plus-square road search smile-o star star-half-o thumb-tack thumbs-down thumbs-up times trash-o user youtube-play"


mkdir -p Svg/font-awesome/
for icon in $icons; do
	sed "s/#fff/#ffffff/g" "FontAwesome/white/svg/$icon.svg" > "Svg/font-awesome/fa-$icon.svg"
done
