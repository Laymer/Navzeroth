//! `guide verify`: replay an RXP-format guide against a facts file (task V2.3).
//!
//! Both files are explicitly selected and read through the bounded, component-checked
//! local-source boundary. The guide is parsed as data; its Lua is never executed.
//! The JSON report carries line numbers, quest IDs and check names, never guide text.
use azeroth_ingest::{
    rxp::{self, Character, Class, Race},
    rxp_verify::{self, JsonFacts, Report},
};
use std::{ffi::OsString, path::PathBuf};

pub const USAGE: &str = "guide verify --guide <file> --facts <file> [--race <Race>] [--class <Class>]. \
    The character defaults to the first guide's #defaultfor. Exit 0 only when no error, unverified or unsupported finding remains. \
    See docs/data/GUIDE_VERIFIER.md.";

#[derive(Debug)]
pub struct Options {
    guide: PathBuf,
    facts: PathBuf,
    race: Option<Race>,
    class: Option<Class>,
}
impl Options {
    /// `verify` followed by exact option/value pairs, each at most once.
    pub fn parse(args: &[OsString]) -> Option<Self> {
        let (command, rest) = args.split_first()?;
        if command != "verify" || rest.len() > 8 {
            return None;
        }
        let (pairs, remainder) = rest.as_chunks::<2>();
        if !remainder.is_empty() {
            return None;
        }
        let (mut guide, mut facts, mut race, mut class) = (None, None, None, None);
        for [flag, value] in pairs {
            if value.is_empty() || value.as_encoded_bytes().starts_with(b"--") {
                return None;
            }
            match flag.to_str()? {
                "--guide" if guide.is_none() => guide = Some(PathBuf::from(value)),
                "--facts" if facts.is_none() => facts = Some(PathBuf::from(value)),
                "--race" if race.is_none() => race = Some(Race::from_token(value.to_str()?)?),
                "--class" if class.is_none() => class = Some(Class::from_token(value.to_str()?)?),
                _ => return None,
            }
        }
        Some(Self {
            guide: guide?,
            facts: facts?,
            race,
            class,
        })
    }
}

#[derive(Debug)]
pub enum Error {
    Read(crate::questiedb::Error),
    Guide(rxp::Rejection),
    Facts(rxp_verify::FactsRejection),
    /// No character given and none derivable from the guide.
    Character,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Read(e) => e.fmt(f),
            Self::Guide(e) => e.fmt(f),
            Self::Facts(e) => e.fmt(f),
            Self::Character => f.write_str(
                "No character: pass --race and --class, or give the first guide one #defaultfor race and class",
            ),
        }
    }
}
impl std::error::Error for Error {}

pub fn verify(options: &Options) -> Result<Report, Error> {
    let guide_bytes = crate::questiedb::read_local_source(&options.guide).map_err(Error::Read)?;
    let facts_bytes = crate::questiedb::read_local_source(&options.facts).map_err(Error::Read)?;
    let file = rxp::parse(&guide_bytes).map_err(Error::Guide)?;
    let facts = JsonFacts::parse(&facts_bytes).map_err(Error::Facts)?;
    let default = rxp_verify::default_character(&file);
    let character = Character {
        race: options
            .race
            .or(default.map(|c| c.race))
            .ok_or(Error::Character)?,
        class: options
            .class
            .or(default.map(|c| c.class))
            .ok_or(Error::Character)?,
    };
    Ok(rxp_verify::verify(
        &file,
        &facts,
        character,
        Some(facts.origin),
    ))
}
