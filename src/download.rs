use sha1::{Digest, Sha1};
use snafu::{ResultExt, Snafu};
use std::{
  fs::File,
  io::{Read, Write},
  path::{Path, PathBuf},
};

/// Size of the chunk read from the network before the progress callback is called again.
const CHUNK_SIZE: usize = 64 * 1024;

use crate::metadata::{Metadata, MetadataFile};

#[derive(Debug)]
pub enum DownloadMethod {
  Https,
  Torrent,
}

#[derive(Debug, Snafu)]
pub enum Error {
  #[snafu(display("File not found in item: {}", filename))]
  FileNotFound { filename: String },

  #[snafu(display("All servers failed, last error on {}: {}", url, source))]
  DownloadFailed { url: String, source: reqwest::Error },

  #[snafu(display("Transfer interrupted on {}: {}", url, source))]
  TransferFailed { url: String, source: std::io::Error },

  #[snafu(display("IO error on {}: {}", path.display(), source))]
  Io {
    path: PathBuf,
    source: std::io::Error,
  },

  #[snafu(display("Checksum mismatch: expected {}, got {}", expected, got))]
  ChecksumMismatch { expected: String, got: String },

  #[snafu(display("No sha1 checksum available for {}", filename))]
  ChecksumMissing { filename: String },

  #[snafu(display("Download method not yet implemented"))]
  NotImplemented,
}

type Result<T, E = Error> = std::result::Result<T, E>;

pub struct Download<'a> {
  metadata: &'a Metadata,
  file: &'a MetadataFile,
}

impl<'a> Download<'a> {
  pub fn new(metadata: &'a Metadata, filename: &str) -> Result<Self> {
    let file = metadata
      .files
      .iter()
      .find(|f| f.name == filename)
      .ok_or_else(|| Error::FileNotFound {
        filename: filename.to_string(),
      })?;
    Ok(Self { metadata, file })
  }

  /// Size announced by the item metadata, when it is there.
  pub fn size(&self) -> Option<u64> {
    self.file.size.as_deref().and_then(|s| s.parse().ok())
  }

  pub fn fetch(&self, dest: &Path, method: DownloadMethod) -> Result<()> {
    self.fetch_with_progress(dest, method, |_, _| {})
  }

  /// `progress(read, total)` is called as the body is read, once per 64 KiB chunk.
  /// `total` is `None` when the server does not announce a `Content-Length`.
  ///
  /// On server fallback the destination file is truncated and `read` restarts from 0,
  /// so the callback may see the counter go backwards.
  pub fn fetch_with_progress(
    &self,
    dest: &Path,
    method: DownloadMethod,
    progress: impl FnMut(u64, Option<u64>),
  ) -> Result<()> {
    match method {
      DownloadMethod::Https => self.fetch_https(dest, progress),
      DownloadMethod::Torrent => Err(Error::NotImplemented),
    }
  }

  fn fetch_https(&self, dest: &Path, mut progress: impl FnMut(u64, Option<u64>)) -> Result<()> {
    let urls = self
      .metadata
      .file_urls(&self.file.name)
      .map_err(|_| Error::FileNotFound {
        filename: self.file.name.clone(),
      })?;

    let client = reqwest::blocking::Client::new();
    let mut last_err = None;

    for url in &urls {
      match Self::download_url(&client, url, dest, &mut progress) {
        Ok(()) => return Ok(()),
        Err(e) => last_err = Some(e),
      }
    }

    Err(last_err.unwrap())
  }

  fn download_url(
    client: &reqwest::blocking::Client,
    url: &str,
    dest: &Path,
    progress: &mut impl FnMut(u64, Option<u64>),
  ) -> Result<()> {
    let mut res = client
      .get(url)
      .send()
      .and_then(|r| r.error_for_status())
      .context(DownloadFailedSnafu { url })?;
    let total = res.content_length();
    let mut file = File::create(dest).context(IoSnafu { path: dest })?;

    let mut buffer = vec![0u8; CHUNK_SIZE];
    let mut read = 0u64;
    progress(read, total);
    loop {
      let n = res.read(&mut buffer).context(TransferFailedSnafu { url })?;
      if n == 0 {
        break;
      }
      file
        .write_all(&buffer[..n])
        .context(IoSnafu { path: dest })?;
      read += n as u64;
      progress(read, total);
    }
    Ok(())
  }

  pub fn verify_sha1(&self, dest: &Path) -> Result<()> {
    let expected = self
      .file
      .sha1
      .as_deref()
      .ok_or_else(|| Error::ChecksumMissing {
        filename: self.file.name.clone(),
      })?;

    let mut file = File::open(dest).context(IoSnafu { path: dest })?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 8192];
    loop {
      let n = file.read(&mut buffer).context(IoSnafu { path: dest })?;
      if n == 0 {
        break;
      }
      hasher.update(&buffer[..n]);
    }
    let got = format!("{:x}", hasher.finalize());

    if got != expected {
      return Err(Error::ChecksumMismatch {
        expected: expected.to_string(),
        got,
      });
    }
    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::metadata::Metadata;
  use std::env::temp_dir;

  #[test]
  fn fetch_and_verify() {
    let metadata = Metadata::get("QuakeIiiArenaDemo").unwrap();
    let download = Download::new(&metadata, "Q3ADemo.exe").unwrap();

    let dest = temp_dir().join("Q3ADemo.exe");
    download.fetch(&dest, DownloadMethod::Https).unwrap();
    println!("Downloaded to {}", dest.display());

    download.verify_sha1(&dest).unwrap();
    println!("SHA1 verified.");
  }
}
