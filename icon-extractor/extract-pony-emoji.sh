#!/usr/bin/env sh

if [ ! -e Ponyemoji.ttf ]; then
	wget "https://static.fimfiction.net/fonts/ponyemoji.ttf" -O Ponyemoji.ttf || {
		echo "Failed to download Ponyemoji font from FimFiction."
		exit 1
	}
fi

# cat icons.html | grep -oP '(?<=style="font-family:PonyEmoji;"><span>).*?(?=</span>)' | tr -d '\n'
# Copy output of that /\ and put it in here \/ (Character encoding: Unicode, Delimiter: Space)
# https://www.rapidtables.com/convert/number/ascii-to-hex.html
#
# Couldn't figure out how to do that automatically but w/e
# List of unicode codepoints to extract from font:
codepoints="1F601 1F602 1F603 1F604 1F605 1F606 1F607 1F608 1F609 1F60A 1F60B 1F60C 1F60D 1F60E 1F60F 1F610 1F611 1F612 1F613 1F614 1F615 1F616 1F617 1F618 1F619 1F61A 1F61B 1F61C 1F61D 1F61E 1F61F 1F620 1F621 1F622 1F623 1F624 1F625 1F626 1F627 1F628 1F629 1F62A 1F62B 1F62C 1F62D 1F62E 1F62F 1F630 1F631 1F632 1F633 1F634 1F635 1F636 1F637 1F638 1F639 1F63A 1F63B 1F63C 1F63D 1F63E 1F63F 1F640 1F641 1F642 1F643 1F644"

mkdir -p Svg/pony-emoji/
./extract-pony-emoji.ff Ponyemoji.ttf Svg/pony-emoji/ $codepoints
for file in Svg/pony-emoji/*.svg; do
	sed -i "s/currentColor/#1A1A1A/g" $file
done
