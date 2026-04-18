use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;
use urlencoding::encode;

#[derive(Serialize, Deserialize, Debug)]
pub struct MetadataFile {
  pub name: String,
  pub source: String,
  pub format: String,
  pub mtime: Option<String>,
  pub size: Option<String>,
  pub md5: Option<String>,
  pub crc32: Option<String>,
  pub sha1: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MetadataData {
  pub mediatype: String,
  pub title: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
  #[serde(skip)]
  pub item: String,
  pub created: u64,
  pub d1: String,
  pub d2: String,
  pub dir: String,
  pub files: Vec<MetadataFile>,
  pub files_count: u64,
  pub item_last_updated: u64,
  pub item_size: u64,
  pub metadata: MetadataData,
  pub server: String,
  pub uniq: u64,
  pub workable_servers: Vec<String>,
}

#[derive(Debug, Snafu)]
pub enum Error {
  #[snafu(display("Failed to download {}: {}", filename.display(), source))]
  DownloadFailed {
    filename: PathBuf,
    source: reqwest::Error,
  },

  #[snafu(display("Failed to read received data: {}", source))]
  ParseFailed { source: serde_json::Error },
  #[snafu(display("Failed find file: {}", filename))]
  FileNotFound { filename: String },
}

type Result<T, E = Error> = std::result::Result<T, E>;

impl Metadata {
  pub fn get(item: &str) -> Result<Metadata> {
    let client = reqwest::blocking::Client::new();
    let url = format!("https://archive.org/metadata/{}", item);
    let res = client
      .get(&url)
      .send()
      .and_then(|r| r.error_for_status())
      .context(DownloadFailedSnafu {
        filename: PathBuf::from(&url),
      })?;
    let s = res.text().context(DownloadFailedSnafu {
      filename: PathBuf::from(&url),
    })?;
    let mut metadata: Metadata = serde_json::from_str(&s).context(ParseFailedSnafu)?;
    metadata.item = item.to_string();
    Ok(metadata)
  }

  pub fn torrent_url(&self) -> String {
    format!(
      "https://archive.org/download/{}/{}_archive.torrent",
      self.item, self.item
    )
  }

  pub fn file_exists(&self, filename: &str) -> bool {
    self.files.iter().any(|file| file.name == filename)
  }

  pub fn file_urls(&self, filename: &str) -> Result<Vec<String>> {
    if !self.file_exists(filename) {
      return Err(Error::FileNotFound {
        filename: filename.to_string(),
      });
    }
    let encoded = encode(filename);
    let mut servers = vec![self.d1.clone(), self.d2.clone()];
    for s in &self.workable_servers {
      if *s != self.d1 && *s != self.d2 {
        servers.push(s.clone());
      }
    }
    Ok(
      servers
        .into_iter()
        .map(|s| format!("https://{}{}/{}", s, self.dir, encoded))
        .collect(),
    )
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn it_works() {
    let metadata = Metadata::get("QuakeIiiArenaDemo").unwrap();
    println!("{:#?}", metadata);
    let urls = metadata.file_urls("Q3ADemo.exe").unwrap();
    println!("URLs = {:#?}", urls);
  }

  #[test]
  fn torrent_url() {
    let metadata = Metadata::get("QuakeIiiArenaDemo").unwrap();
    let url = metadata.torrent_url();
    println!("Torrent URL = {}", url);
    assert_eq!(
      url,
      "https://archive.org/download/QuakeIiiArenaDemo/QuakeIiiArenaDemo_archive.torrent"
    );
  }
}
