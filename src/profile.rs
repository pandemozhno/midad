use std::collections::HashMap;
use std::fs;
use crate::error::{Error, ErrorKind, Result};
use crate::token::{Keyword, TypeTok};

pub struct Profile {
    pub keywords: HashMap<String, Keyword>,
    pub types: HashMap<String, TypeTok>,
    pub names: HashMap<String, String>,
    pub lang: String,
}

impl Profile {
    pub fn load(path: &str) -> Result<Profile> {
        let content = fs::read_to_string(path)
            .map_err(|e| Error::new(ErrorKind::Io,
                format!("не могу прочитать {}: {}", path, e), 0, 0))?;
        let mut profile = Profile::parse(&content)?;
        profile.lang = path
            .rsplit('/').next()
            .and_then(|f| f.strip_suffix(".sema"))
            .unwrap_or("en")
            .to_string();
        Ok(profile)
    }

    pub fn parse(content: &str) -> Result<Profile> {
        let mut keywords = HashMap::new();
        let mut types = HashMap::new();
        let mut names = HashMap::new();
        let mut section = "";

        for (ln, raw) in content.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = &line[1..line.len() - 1];
                continue;
            }
            let (a, b) = match line.split_once('=') {
                Some((a, b)) => (a.trim(), b.trim()),
                None => return Err(Error::new(ErrorKind::Io,
                    format!("плохая строка в .sema: '{}'", line), ln + 1, 0)),
            };
            match section {
                "keywords" => {
                    if let Some(kw) = Keyword::from_canonical(a) {
                        keywords.insert(b.to_string(), kw);
                    }
                }
                "types" => {
                    if let Some(t) = TypeTok::from_canonical(a) {
                        types.insert(b.to_string(), t);
                    }
                }
                "names" => {
                    // canonical=localized → храню localized → canonical
                    names.insert(b.to_string(), a.to_string());
                }
                _ => {}
            }
        }

        Ok(Profile { keywords, types, names, lang: String::new() })
    }
}