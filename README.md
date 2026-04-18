# internetarchive

Unofficial Rust library for [archive.org](https://archive.org).

## Features

- Fetch item metadata (files list, servers, checksums)
- Build download URLs for a specific file, across all available servers

## Usage

```rust
use internet_archive::metadata::Metadata;

let metadata = Metadata::get("QuakeIiiArenaDemo")?;

// Check if a file exists in the item
if metadata.file_exists("Q3ADemo.exe") {
    // Returns URLs for all available servers (d1 first, d2 second, then workable_servers)
    let urls = metadata.file_urls("Q3ADemo.exe")?;
    for url in urls {
        println!("{}", url);
    }
}
```

## Changelog

### 0.2.0 — Breaking changes

- **`fileurl_get` renamed to `file_urls`**, now returns `Result<Vec<String>>` instead of `Result<String>`.
  Previously only the primary server (`d1`) was returned. It now returns URLs for all
  available servers: `d1` first, `d2` second, then the remaining `workable_servers`.
  This allows callers to implement fallback or load balancing strategies.

- **`file_exist` renamed to `file_exists`** — idiomatic Rust predicate naming.

- **`torrent_url()` added** — returns the `.torrent` URL for the item, usable with an external client such as `aria2c`.

- **`Metadata::item` field added** — the item identifier is now stored in the struct after `get()`.

- Switched TLS backend from OpenSSL to `rustls`. No system dependency required.

- Removed redundant `serde_derive` dependency.

### 0.1.x

Initial release.
