# Truth Social investigator

[![build](https://github.com/travisbrown/truthsocial/actions/workflows/ci.yml/badge.svg)](https://github.com/travisbrown/truthsocial/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/travisbrown/truthsocial/branch/main/graph/badge.svg)](https://codecov.io/gh/travisbrown/truthsocial)
[![crates.io](https://img.shields.io/crates/v/truthsocial.svg)](https://crates.io/crates/truthsocial)
[![docs.rs](https://docs.rs/truthsocial/badge.svg)](https://docs.rs/truthsocial)

A Rust project for collecting and processing [Truth Social][truth-social] data, both from the live
([Mastodon][mastodon]-compatible) HTTP API and from [Wayback Machine][wayback-machine] archives.

## Crates

| Crate                            | Description                                                                                                                    |
| -------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ |
| [`truthsocial`](core)            | Core types modeling the Truth Social API (statuses, accounts, tags, groups).                                                   |
| [`truthsocial-api`](api)         | HTTP client for the live API.                                                                                                  |
| [`truthsocial-wbm`](wbm)         | Wayback Machine snapshot collection and management.                                                                            |
| [`truthsocial-api-cli`](api-cli) | The `truthsocial-api` command-line tool: browser-assisted login with account and status lookups.                               |
| [`truthsocial-wbm-cli`](wbm-cli) | The `truthsocial-wbm` command-line tool: batch processing over archive datasets (packing, enhancing, and validating captures). |

## Building

```bash
cargo build --release
```

`truthsocial-api-cli` pulls in [chromiumoxide][chromiumoxide] and is excluded from the default set. Build it explicitly:

```bash
cargo build --release -p truthsocial-api-cli
```

## License

This project is licensed under the [GNU Affero General Public License, version 3
only](https://www.gnu.org/licenses/agpl-3.0.html). See [LICENSE](LICENSE) for the full text.

[archivindex]: https://github.com/travisbrown/archivindex
[chromiumoxide]: https://github.com/mattsse/chromiumoxide
[mastodon]: https://joinmastodon.org/
[truth-social]: https://truthsocial.com/
[wayback-machine]: https://web.archive.org/
