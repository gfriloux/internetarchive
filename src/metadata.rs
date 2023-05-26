use serde_derive::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;
use urlencoding::encode;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MetadataFile {
	pub name: String,
	pub source: String,
	pub format: String,
	pub mtime: Option<String>,
	pub size: Option<String>,
	pub md5: Option<String>,
	pub crc32: Option<String>,
	pub sha1: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MetadataData {
	pub mediatype: String,
	pub title: String
	
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Metadata {
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
	pub workable_servers: Vec<String>
}

#[derive(Debug, Snafu)]
pub enum Error {
   #[snafu(display("Failed to download {}: {}", filename.display(), source))]
   DownloadFailed {
      filename: PathBuf,
      source: reqwest::Error,
   },

   #[snafu(display("Failed to read received data: {}", source))]
   ParseFailed {
      source: serde_json::Error,
   },
  #[snafu(display("Failed find file: {}", filename))]
  FileNotFound {
    filename: String,
  }
}

type Result<T, E = Error> = std::result::Result<T, E>;

impl Metadata {
	pub fn get(item: &str) -> Result<Metadata> {
    let client = reqwest::blocking::Client::new();
    let url = format!("https://archive.org/metadata/{}", item);
    let res = client.get(&url)
                    .send()
                    .context(DownloadFailedSnafu { filename: PathBuf::from(&url) })?;
    let s = res.text().context(DownloadFailedSnafu { filename: PathBuf::from(&url) })?;
    println!("{}", s);
    let metadata: Metadata = serde_json::from_str(&s).context(ParseFailedSnafu)?;
    Ok(metadata)
	}

	pub fn file_exist(&self, filename: &str) -> bool {
	  let mut result = false;
	  if self.files.clone().into_iter().any(|file| file.name.eq(filename)) {
	    result = true;
	  }
	  result
	}

	pub fn fileurl_get(&self, filename: &str) -> Result<String> {
	  // Making sure file exists
	  if ! self.file_exist(filename) {
	    return Err(Error::FileNotFound { filename: filename.to_string() })
	  }
	  let url = format!("https://{}{}/{}", self.d1, self.dir, encode(filename));
    Ok(url)
	}
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
      let metadata = Metadata::get("QuakeIiiArenaDemo").unwrap();
      println!("{:#?}", metadata);
      let url = metadata.fileurl_get("Q3ADemo.exe").unwrap();
      println!("URL = {}", url);
    }
}
