# Distribution

Quickfox ships as one native binary. There are no published releases or packages yet.

## Linux archive

On Linux x86_64:

```bash
bash scripts/package-linux.sh
python3 scripts/smoke_pty.py target/release/quickfox
(cd target/packages && sha256sum --check SHA256SUMS)
```

This writes `target/packages/quickfox-<version>-linux-x86_64.tar.gz` and `SHA256SUMS`.
The archive holds the executable, README, changelog, docs and screenshots, the config
example and schema, the word-list provenance and the optional sound installer. The
personal-use keyboard recordings are never included.

The binary links against the build machine's glibc; it is not a static or musl build.
Key sounds load `libpulse-simple.so.0` at runtime when available, and the game starts
without it. Check runtime libraries with `ldd` and `readelf`.

The release workflow runs on version tags or by hand. It builds on Ubuntu 24.04 with
Rust 1.88, runs the PTY smoke tests against the release binary and uploads the archive
as a workflow artifact. Checksums catch corrupted downloads; they are not signatures.

## Installing an archive

```bash
sha256sum --check SHA256SUMS
tar -xzf quickfox-<version>-linux-x86_64.tar.gz
install -Dm755 quickfox-<version>-linux-x86_64/quickfox ~/.local/bin/quickfox
quickfox --version
```

Add `~/.local/bin` to `PATH` if needed. To uninstall, delete that file, or run
`cargo uninstall quickfox` for a Cargo install. Your saves stay until you remove them;
`quickfox doctor` shows where they are.

## Later

Prebuilt GitHub releases, cargo-dist installers, an npm wrapper around the native
binary, and macOS, Windows, musl and ARM builds.
