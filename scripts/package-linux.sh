#!/usr/bin/env bash
set -euo pipefail
if [[ "$(uname -s)" != Linux || "$(uname -m)" != x86_64 ]]; then
  echo 'This packaging script targets Linux x86_64 only.' >&2
  exit 1
fi
cargo build --release --locked
version=$(target/release/quickfox --version | awk '{print $2}')
name="quickfox-${version}-linux-x86_64"
package_stage=$(mktemp -d)
trap 'rm -rf "$package_stage"' EXIT
mkdir -p "target/packages" "$package_stage/$name"
cp target/release/quickfox README.md CHANGELOG.md LICENSE "$package_stage/$name/"
cp -R docs "$package_stage/$name/docs"
cp -R config "$package_stage/$name/config"
mkdir -p "$package_stage/$name/assets/words"
cp assets/words/LICENSES.md "$package_stage/$name/assets/words/LICENSES.md"
tar -C "$package_stage" -czf "target/packages/$name.tar.gz" "$name"
(cd target/packages && sha256sum "$name.tar.gz" > SHA256SUMS)
echo "Packaged target/packages/$name.tar.gz"
