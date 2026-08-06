#!/usr/bin/env bash
# Build everything the box needs, into one directory.
#
#   ./scripts/build-release.sh            # for this machine
#   TARGET=aarch64-unknown-linux-gnu ./scripts/build-release.sh
#
# **Run it on the box for the first deploy.** Cross-compiling is the faster answer once it is
# set up and one more thing to get wrong when nothing has ever worked yet; the box has 24GB and
# four cores, and a release build there takes about ten minutes. CI already proves the target
# compiles on every commit, so this is about convenience rather than risk.
#
# The output is `dist/`: a binary, the built web app, and nothing else. Everything
# environmental lives in `/etc/landscape/landscape.env` on the box — see `deploy/`.
set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$here"
out="$here/dist"

target_args=()
bin_dir="target/release"
if [[ -n "${TARGET:-}" ]]; then
    target_args=(--target "$TARGET")
    bin_dir="target/$TARGET/release"
fi

echo "==> web"
# The API serves these files; a missing `dist` means every page is a 404 that looks like a
# routing bug. Built first so a frontend failure stops the build before the slow half.
(cd web && npm ci && npm run build)

echo "==> binary"
cargo build --release -p landscape "${target_args[@]}"

echo "==> assembling dist/"
rm -rf "$out"
mkdir -p "$out/bin" "$out/web"
cp "$bin_dir/landscape" "$out/bin/landscape"
cp -r web/dist "$out/web/dist"

# Which commit this is. A box serving a report nobody can reproduce is the thing ADR 0005's
# request ids exist to prevent, and this is the same argument one level up.
git rev-parse HEAD > "$out/COMMIT"

echo
echo "dist/ ready:"
find "$out" -maxdepth 2 -mindepth 1 -printf '  %P\n' | sort
echo
echo "Next: docs/DEPLOY.md step 5."
