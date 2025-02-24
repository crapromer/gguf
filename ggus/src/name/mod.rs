﻿mod shard;
mod size_label;
mod r#type;
mod version;

use shard::Shard;
use size_label::SizeLabel;
use std::{borrow::Cow, fmt, num::NonZero, path::Path};
use r#type::Type;
use version::Version;

#[derive(Clone, Debug)]
pub struct GGufFileName<'a> {
    pub base_name: Cow<'a, str>,
    pub size_label: Option<SizeLabel>,
    pub fine_tune: Cow<'a, str>,
    pub version: Version,
    pub encoding: Option<Cow<'a, str>>,
    pub type_: Type,
    pub shard: Shard,
}

mod pattern {
    use regex::Regex;
    use std::sync::LazyLock;

    pub const NAME_: &str = r"-(\d+x)?(\d+)(\.\d+)?([QTBMK])(-\w+)?$";
    pub const VERSION_: &str = r"-v(\d+)\.(\d+)$";
    pub const TYPE_LORA: &str = r"-LoRA";
    pub const TYPE_VOCAB: &str = r"-vocab";
    pub const SHARD_: &str = r"-(\d{5})-of-(\d{5})$";
    pub const EXT: &str = ".gguf";

    pub static NAME: LazyLock<Regex> = LazyLock::new(|| Regex::new(NAME_).unwrap());
    pub static VERSION: LazyLock<Regex> = LazyLock::new(|| Regex::new(VERSION_).unwrap());
    pub static SHARD: LazyLock<Regex> = LazyLock::new(|| Regex::new(SHARD_).unwrap());
}

#[derive(Debug)]
pub struct GGufExtNotMatch;

impl<'a> TryFrom<&'a str> for GGufFileName<'a> {
    type Error = GGufExtNotMatch;

    fn try_from(name: &'a str) -> Result<Self, Self::Error> {
        let Some(mut name) = name.strip_suffix(pattern::EXT) else {
            return Err(GGufExtNotMatch);
        };

        let shard = pattern::SHARD
            .captures(name)
            .map_or_else(Shard::default, |capture| {
                let (full, [index, count]) = capture.extract();
                name = &name[..name.len() - full.len()];
                Shard::new(index.parse().unwrap(), count.parse().unwrap())
            });

        let name_no_shard = name;
        let type_ = if name.ends_with(pattern::TYPE_VOCAB) {
            name = &name[..name.len() - pattern::TYPE_VOCAB.len()];
            Type::Vocab
        } else if name.ends_with(pattern::TYPE_LORA) {
            name = &name[..name.len() - pattern::TYPE_LORA.len()];
            Type::LoRA
        } else {
            Type::Default
        };

        let Some((head, encoding)) = name.rsplit_once('-') else {
            return Ok(Self {
                base_name: name_no_shard.into(),
                size_label: None,
                fine_tune: "".into(),
                version: Version::DEFAULT,
                encoding: None,
                type_,
                shard,
            });
        };
        name = head;

        let version = pattern::VERSION
            .captures(name)
            .map_or(Version::DEFAULT, |capture| {
                let (full, [major, minor]) = capture.extract();
                name = &name[..name.len() - full.len()];
                Version::new(major.parse().unwrap(), minor.parse().unwrap())
            });

        if let Some(capture) = pattern::NAME.captures(name) {
            let base_name = &name[..name.len() - capture.get(0).unwrap().len()];
            let e = capture.get(1).map_or(1, |m| {
                m.as_str().strip_suffix('x').unwrap().parse().unwrap()
            });
            let a = capture.get(2).unwrap().as_str().parse().unwrap();
            let b = capture.get(3).map_or(0, |m| {
                m.as_str().strip_prefix('.').unwrap().parse().unwrap()
            });
            let l = capture.get(4).unwrap().as_str().chars().next().unwrap();
            let fine_tune = capture
                .get(5)
                .map_or("", |m| m.as_str().strip_prefix('-').unwrap());

            Ok(Self {
                base_name: base_name.into(),
                size_label: Some(SizeLabel::new(e, a, b, l)),
                fine_tune: fine_tune.into(),
                version,
                encoding: Some(encoding.into()),
                type_,
                shard,
            })
        } else {
            Ok(Self {
                base_name: name_no_shard.into(),
                size_label: None,
                fine_tune: "".into(),
                version: Version::DEFAULT,
                encoding: None,
                type_,
                shard,
            })
        }
    }
}

impl<'a> TryFrom<&'a Path> for GGufFileName<'a> {
    type Error = GGufExtNotMatch;
    #[inline]
    fn try_from(value: &'a Path) -> Result<Self, Self::Error> {
        Self::try_from(value.file_name().unwrap().to_str().unwrap())
    }
}

impl GGufFileName<'_> {
    #[inline]
    pub fn shard_count(&self) -> usize {
        self.shard.count.get() as _
    }

    #[inline]
    pub fn into_single(self) -> Self {
        Self {
            shard: Default::default(),
            ..self
        }
    }

    #[inline]
    pub fn iter_all(self) -> Self {
        Self {
            shard: Shard {
                index: NonZero::new(1).unwrap(),
                ..self.shard
            },
            ..self
        }
    }

    #[inline]
    pub fn split_n(self, n: usize) -> Self {
        Self {
            shard: Shard {
                index: NonZero::new(1).unwrap(),
                count: NonZero::new(n as _).unwrap(),
            },
            ..self
        }
    }
}

impl Iterator for GGufFileName<'_> {
    type Item = Self;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.shard.index.get() <= self.shard.count.get() {
            let ans = self.clone();
            self.shard.index = self.shard.index.checked_add(1).unwrap();
            Some(ans)
        } else {
            None
        }
    }
}

impl fmt::Display for GGufFileName<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.base_name)?;
        if let Some(size_label) = &self.size_label {
            write!(f, "-{size_label}")?
        }
        if !self.fine_tune.is_empty() {
            write!(f, "-{}", self.fine_tune)?
        }
        write!(f, "-{}", self.version)?;
        if let Some(encoding) = &self.encoding {
            write!(f, "-{}", encoding)?
        }
        write!(f, "{}", self.type_)?;
        write!(f, "{}", self.shard)?;
        write!(f, ".gguf")
    }
}

#[test]
fn test_name() {
    fn check(name: &str) {
        println!("{name} -> {}", GGufFileName::try_from(name).unwrap())
    }

    check("mmproj.gguf");
    check("FM9G-71B-F16.gguf");
    check("test-cases-00002-of-00005.gguf");
    check("Gpt-163M-v2.0-F32.gguf");
    check("TinyLlama-1.1B-Chat-v1.0-Q8_0.gguf");
    check("MiniCPM3-1B-sft-v0.0-F16.gguf");
    check("MiniCPM-V-Clip-1B-v2.6-F16.gguf");
}
