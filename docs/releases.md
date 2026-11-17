# Building a release

The `Build release artifacts` GitHub Actions workflow runs manually from Actions
or when a `v*` tag is pushed. It has read-only repository permissions and does
not create a GitHub release, upload release attachments, or publish to crates.io.

## Before building

- Keep LICENSE and ARTWORK-LICENSE.md with distributions. The code is MIT-licensed
  to Cody-will and contributors; artwork provenance and third-party rights are
  documented separately. Review those terms when adding new artwork.
- Review third-party licenses, including bundled native code. The packaging
  script collects license/notice files and dependency metadata as review material;
  it is not a determination that every dependency obligation has been met.
  The archive includes unmodified MPL-2.0 dependency source, and a supplemental
  upstream MIT notice for quantette 0.6.0. Packaging fails on missing notices.
- Set the desired version in Cargo.toml and refresh Cargo.lock with Cargo.
- Update CHANGELOG.md, then commit the complete source and assets.

## Build

For a local native Linux x86_64 build with Python 3.11 or newer:

```sh
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo build --release --locked
python3 scripts/package-release.py
```

The workflow runs those checks on Ubuntu 24.04 with stable Rust and the required
XCB/GLib development packages. A version tag must match Cargo.toml exactly.
The produced executable is dynamically linked; use BUILD-INFO.txt for the compiler,
build platform, and shared-library requirements. Do not advertise universal Linux
compatibility without testing the intended distributions.

The script writes a versioned tar.gz and a matching .sha256 file to `dist/`.
Archives include the executable, documentation, examples, screenshots, license
files, dependency notices/inventory, and build information. Artwork is embedded.
Run the archive on a clean target system and check graphics in real terminals.

## Publish manually

1. Push your prepared commit. Run the workflow manually to check it before tagging.
2. Push the matching version tag when ready, for example `v0.1.0`.
3. Open the successful Actions run and download the `blink-linux-x86_64` artifact.
4. Extract the Actions ZIP; it contains the tar.gz and checksum file to distribute.
5. Verify with `sha256sum -c blink-v0.1.0-x86_64-unknown-linux-gnu.sha256`.
6. Create the GitHub release yourself, select the matching tag, paste reviewed
   changelog notes, and attach the tar.gz and .sha256 files. Describe tested systems
   and runtime requirements, then publish when ready.

Actions artifacts expire after 30 days. Release attachments remain available
according to GitHub's release retention policies. No extra publishing token is
required by this workflow because it only uploads Actions artifacts.
