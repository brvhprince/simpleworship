# Bundled Bibles

SimpleWorship ships with two public-domain English Bibles, the Asante Twi
Bible and the French Louis Segond. They are imported into the library on
first launch, using the same USFM importer as any Bible a user imports. Many
more can be downloaded from the app (Manage Bibles › Get More Bibles), from
the same source.

| File                   | Translation                         | Source                                  | Licence |
| ---------------------- | ----------------------------------- | --------------------------------------- | ------- |
| `eng-kjv2006_usfm.zip` | King James Version (1769 text)      | <https://ebible.org/find/details.php?id=eng-kjv2006> | Public domain¹ |
| `engwebp_usfm.zip`     | World English Bible                 | <https://ebible.org/find/details.php?id=engwebp>     | Public domain² |
| `twiasante_usfm.zip`   | Biblica® Open Asante Twi Contemporary Bible (Asante Twi Nkwa Asɛm) | <https://ebible.org/find/details.php?id=twiasante> | CC BY-SA 4.0³ |
| `fraLSG_usfm.zip`      | Louis Segond 1910 (French)          | <https://ebible.org/find/details.php?id=fraLSG>      | Public domain |

¹ The KJV is in the public domain everywhere except the United Kingdom, where
a royal letters patent restricts printing and importing printed copies. Users
in the UK who are unsure what that means for them should check for
themselves; see the source page above for details.

² "World English Bible" is a trademark of eBible.org: if the text is changed,
the result must not be called the World English Bible.

³ © 1996, 2020 Biblica, Inc., under the Creative Commons
Attribution-ShareAlike 4.0 International licence
(<http://creativecommons.org/licenses/by-sa/4.0>). It is shipped unchanged,
under its own name with the Biblica® trademark, and its copyright line is
shown with its verses, as the licence asks. "Biblica" is a trademark of
Biblica, Inc.; a changed or translated copy must not carry it.

The zips are kept exactly as downloaded (each also contains eBible.org's
`copr.htm` licence page). To update them, run `scripts/fetch-bibles.sh`.

## Checksums (SHA-256)

```
16b08c9248b3600662f66f1e055f61fc3a6c7275e347236ee45feb35dfb20cb3  eng-kjv2006_usfm.zip
732c17d3325563fc5d8d98157a6c3d128c887f6b8e5a5ad0f71f58e5d5eebfcb  engwebp_usfm.zip
1e07cd2c088416e9b3b1a87fa6a5fbec5225e59cc989b414ae8d910455b875e4  twiasante_usfm.zip
95dcf4a7ca9992bc829baddbc6f5ee8e1d96e34dac099e53cbfac5adb60ef250  fraLSG_usfm.zip
```

KJV and WEB downloaded on 2026-10-19; Asante Twi and Louis Segond on
2026-10-26.

## Other translations

Most modern translations (NIV, ESV, NKJV, NLT, …) are copyrighted and
need a licence from their publisher to be shown in public. SimpleWorship
never bundles them. Users may import Bible files they are licensed to use.
