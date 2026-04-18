use sha1::{Digest, Sha1};
use snafu::{ResultExt, Snafu};
use std::{
  fs::File,
  io::Read,
  path::{Path, PathBuf},
};

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

  pub fn fetch(&self, dest: &Path, method: DownloadMethod) -> Result<()> {
    match method {
      DownloadMethod::Https => self.fetch_https(dest),
      DownloadMethod::Torrent => Err(Error::NotImplemented),
    }
  }

  fn fetch_https(&self, dest: &Path) -> Result<()> {
    let urls = self
      .metadata
      .file_urls(&self.file.name)
      .map_err(|_| Error::FileNotFound {
        filename: self.file.name.clone(),
      })?;

    let client = reqwest::blocking::Client::new();
    let mut last_err = None;

    for url in &urls {
      match Self::download_url(&client, url, dest) {
        Ok(()) => return Ok(()),
        Err(e) => last_err = Some(e),
      }
    }

    Err(last_err.unwrap())
  }

  fn download_url(client: &reqwest::blocking::Client, url: &str, dest: &Path) -> Result<()> {
    let mut res = client
      .get(url)
      .send()
      .and_then(|r| r.error_for_status())
      .context(DownloadFailedSnafu { url })?;
    let mut file = File::create(dest).context(IoSnafu { path: dest })?;
    res
      .copy_to(&mut file)
      .context(DownloadFailedSnafu { url })?;
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
