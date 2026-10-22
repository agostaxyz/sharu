#!/usr/bin/env sh
# Downloads the official AGPL-3.0 text (we cannot vendor it from memory reliably).
set -eu
mkdir -p LICENSES
curl -fsSL https://www.gnu.org/licenses/agpl-3.0.txt -o LICENSE
cp LICENSE LICENSES/AGPL-3.0-or-later.txt
echo "LICENSE written. Commit it."
