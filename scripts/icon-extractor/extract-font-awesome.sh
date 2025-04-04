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
icons="bolt book bookmark certificate check times clock-o cog exclamation eye film gamepad globe heart lock music road search star thumbs-up thumbs-down thumb-tack smile-o meh-o frown-o trash-o user youtube-play "

mkdir -p Svg/font-awesome/
for icon in $icons; do
	sed "s/#fff/#ffffff/g" "FontAwesome/white/svg/$icon.svg" > "Svg/font-awesome/fa-$icon.svg"
done