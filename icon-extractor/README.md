Extracts the fimfiction bookshelf icons from font awesome and the pony font. To use, cd into this directory and run `./extract.sh`. The icons will be in `Out/[color]/[font-awesome|pony-font]/[font awesome name|unicode codepoint in hex].svg`

Original PonyFont used by fimfiction: <https://www.reddit.com/r/mylittlepony/comments/28vht4/ive_finished_my_mlp_emoji_pack_infos_and_download/>

Script to extract SVG from TTF using Fontforge: <https://barrd.dev/article/convert-all-glyphs-of-a-font-to-individual-svg-files/>

Fimfic uses FontAwesome 4 which doesn't seem to provide a SVG download, so I'm using [Font-Awesome-SVG-PNG](https://github.com/Rush/Font-Awesome-SVG-PNG/releases/tag/1.1.5)