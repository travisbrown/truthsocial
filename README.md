# Truth Social investigator

![GitHub last commit][last-commit-badge]
[![build][build-badge]][build]
[![codecov][codecov-badge]][codecov]
[![license][license-badge]][agpl-3.0]
[![crates.io][crates-version-badge]][crates]
[![crates.io][crates-downloads-badge]][crates]
[![API Docs][docs-badge]][docs]

A Rust project for collecting and processing [Truth Social][truth-social] data, both from the live
HTTP API (based on [Mastodon][mastodon]) and from [Wayback Machine][wayback-machine] archives.

## Crates

| Crate | Description |
| --- | --- |
| [`truthsocial`](crates/core/) | Core types for statuses, accounts, tags, and groups |
| [`truthsocial-api`](crates/api/) | HTTP client for the live API |
| [`truthsocial-wbm`](crates/wbm/) | Wayback Machine snapshot reading, validation, and merging |
| [`truthsocial-api-cli`](tools/api-cli/) | Browser-assisted login, account and status lookups, group queries, and search |
| [`truthsocial-wbm-cli`](tools/wbm-cli/) | Archive validation, compaction, merging, and CDX processing |

## Building

```bash
cargo build --release
```

`truthsocial-api-cli` uses [chromiumoxide][chromiumoxide] to drive a browser and is excluded from
the default build. Build it explicitly:

```bash
cargo build --release -p truthsocial-api-cli
```

## Development

The workspace requires Rust 1.98 or later. Run its tests and build its documentation with:

```console
cargo test --locked --workspace --all-features
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --all-features --no-deps
```

## License

This project is licensed under the [GNU Affero General Public License, version 3
only](https://www.gnu.org/licenses/agpl-3.0.html). See [LICENSE](LICENSE) for the full text.

[agpl-3.0]: https://www.gnu.org/licenses/agpl-3.0.html
[build]: https://github.com/travisbrown/truthsocial/actions/workflows/ci.yml
[build-badge]: https://github.com/travisbrown/truthsocial/actions/workflows/ci.yml/badge.svg
[chromiumoxide]: https://github.com/mattsse/chromiumoxide
[codecov]: https://codecov.io/gh/travisbrown/truthsocial
[codecov-badge]: https://codecov.io/gh/travisbrown/truthsocial/branch/main/graph/badge.svg
[crates]: https://crates.io/crates/truthsocial/
[crates-downloads-badge]: https://img.shields.io/crates/d/truthsocial
[crates-version-badge]: https://img.shields.io/crates/v/truthsocial.svg
[docs]: https://docs.rs/truthsocial/
[docs-badge]: https://docs.rs/truthsocial/badge.svg
[last-commit-badge]: https://img.shields.io/github/last-commit/travisbrown/truthsocial
[license-badge]: https://img.shields.io/badge/license-AGPL--v3-blue
[mastodon]: https://joinmastodon.org/
[truth-social]: https://truthsocial.com/
[wayback-machine]: https://web.archive.org/
