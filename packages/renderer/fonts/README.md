# Bundled fonts

These fonts ship with SimpleWorship so themes look the same on every
computer. All are licensed under the [SIL Open Font License 1.1](https://openfontlicense.org)
(see each folder's `OFL.txt`) and come from the
[Google Fonts repository](https://github.com/google/fonts/tree/main/ofl).

| Family       | Files                                   |
| ------------ | --------------------------------------- |
| Montserrat   | variable weight, upright and italic     |
| Open Sans    | variable weight, upright and italic     |
| Merriweather | variable weight, upright and italic¹    |
| Lato         | regular, italic, bold, bold italic      |
| Bebas Neue   | regular                                 |

To keep the download small, every font was converted to WOFF2 and trimmed
to Latin scripts (Basic Latin, Latin-1, Latin Extended A/B/C/D and
Additional, IPA extensions, combining marks, common punctuation and
currency signs). That covers English and most African and European languages
written in Latin script, including letters such as Ɛ ɛ Ɔ ɔ Ŋ ŋ. Text in
other scripts still displays, using the computer's own fonts.

¹ Merriweather's optical-size and width axes were fixed at 18 pt and 100%,
which cut its size by two thirds.

The conversion used [fontTools](https://github.com/fonttools/fonttools):

```sh
fonttools varLib.instancer Merriweather[opsz,wdth,wght].ttf opsz=18 wdth=100
pyftsubset FONT.ttf --flavor=woff2 --layout-features='*' \
  --unicodes="U+0000-036F,U+1AB0-1AFF,U+1DC0-1DFF,U+1E00-1EFF,U+2000-206F,\
U+2070-209F,U+20A0-20CF,U+2100-218F,U+2190-21FF,U+2C60-2C7F,U+A720-A7FF,\
U+FB00-FB06,U+FEFF,U+FFFD"
```
