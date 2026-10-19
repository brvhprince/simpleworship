#!/usr/bin/env bash
# Downloads the public-domain Bibles bundled with SimpleWorship from
# eBible.org into resources/bibles/. See resources/bibles/README.md.
#
# Usage: scripts/fetch-bibles.sh
set -euo pipefail

cd "$(dirname "$0")/../resources/bibles"

for id in eng-kjv2006 engwebp; do
  echo "Downloading ${id}…"
  curl --fail --location --silent --show-error \
    --output "${id}_usfm.zip" "https://ebible.org/Scriptures/${id}_usfm.zip"
done

shasum -a 256 ./*_usfm.zip
echo "Done. Update the checksums in README.md, then run:"
echo "  cargo test --release -p sw-bible --test real_bibles -- --ignored"
