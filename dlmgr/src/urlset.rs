use std::collections::HashSet;
use url::Url;

#[derive(Clone)]
pub struct UrlSet {
    pub urls: Vec<Url>,
}

impl UrlSet {
    pub fn all(&self) -> HashSet<Url> {
        self.urls.iter().cloned().collect()
    }

    pub fn url(&self, worker_nr: u8, attempt_nr: usize) -> Url {
        let url_idx = attempt_nr.overflowing_add(worker_nr as usize).0 % self.urls.len();
        self.urls[url_idx].clone()
    }
}

impl TryFrom<String> for UrlSet {
    type Error = url::ParseError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Ok(Self {
            urls: vec![Url::parse(&value)?],
        })
    }
}

impl From<Url> for UrlSet {
    fn from(url: Url) -> Self {
        Self { urls: vec![url] }
    }
}

impl FromIterator<Url> for UrlSet {
    fn from_iter<I: IntoIterator<Item = Url>>(iter: I) -> Self {
        UrlSet {
            urls: iter.into_iter().collect(),
        }
    }
}
