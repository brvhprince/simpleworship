# Bundled Bibles

SimpleWorship ships with two public-domain English Bibles. They are imported
into the library on first launch, using the same USFM importer as any Bible
a user imports.

| File                   | Translation                         | Source                                  | Licence |
| ---------------------- | ----------------------------------- | --------------------------------------- | ------- |
| `eng-kjv2006_usfm.zip` | King James Version (1769 text)      | <https://ebible.org/find/details.php?id=eng-kjv2006> | Public domain¹ |
| `engwebp_usfm.zip`     | World English Bible                 | <https://ebible.org/find/details.php?id=engwebp>     | Public domain² |

¹ The KJV is in the public domain everywhere except the United Kingdom, where
a royal letters patent restricts printing and importing printed copies. Users
in the UK who are unsure what that means for them should check for
themselves; see the source page above for details.

² "World English Bible" is a trademark of eBible.org: if the text is changed,
the result must not be called the World English Bible.

The zips are kept exactly as downloaded (each also contains eBible.org's
`copr.htm` licence page). To update them, run `scripts/fetch-bibles.sh`.

## Checksums (SHA-256)

```
16b08c9248b3600662f66f1e055f61fc3a6c7275e347236ee45feb35dfb20cb3  eng-kjv2006_usfm.zip
732c17d3325563fc5d8d98157a6c3d128c887f6b8e5a5ad0f71f58e5d5eebfcb  engwebp_usfm.zip
```

Downloaded on 2026-10-19.

## Other translations

Most modern translations (NIV, ESV, NKJV, NLT, …) are copyrighted and
need a licence from their publisher to be shown in public. SimpleWorship
never bundles them. Users may import Bible files they are licensed to use.
