//! Command-line options.

use crate::i18n::Lang;
use crate::sim::Difficulty;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub lang: Option<Lang>,
    pub difficulty: Option<Difficulty>,
    pub seed: Option<u64>,
    pub headless: bool,
    pub seeds: Option<u32>,
    pub autoplay: bool,
    pub bot: Option<String>,
    pub help: bool,
    pub version: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CliError {
    Unknown(String),
    Missing(String),
    Invalid(String, String),
}

pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Options, CliError> {
    let mut o = Options::default();
    let mut it = args.into_iter();
    while let Some(arg) = it.next() {
        let (name, inline) = match arg.split_once('=') {
            Some((n, v)) if n.starts_with("--") => (n.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut value = |name: &str| -> Result<String, CliError> {
            match inline.clone() {
                Some(v) => Ok(v),
                None => it.next().ok_or_else(|| CliError::Missing(name.to_string())),
            }
        };
        match name.as_str() {
            "--lang" | "-l" => {
                let v = value(&name)?;
                o.lang = Some(Lang::from_code(&v).ok_or(CliError::Invalid(name.clone(), v))?);
            }
            "--difficulty" | "-d" => {
                let v = value(&name)?;
                o.difficulty = Some(Difficulty::from_code(&v).ok_or(CliError::Invalid(name.clone(), v))?);
            }
            "--seed" | "-s" => {
                let v = value(&name)?;
                o.seed = Some(v.parse().map_err(|_| CliError::Invalid(name.clone(), v))?);
            }
            "--seeds" => {
                let v = value(&name)?;
                let n: u32 = v.parse().map_err(|_| CliError::Invalid(name.clone(), v.clone()))?;
                if n == 0 {
                    return Err(CliError::Invalid(name, v));
                }
                o.seeds = Some(n);
            }
            "--bot" => {
                let v = value(&name)?;
                if crate::bot::BotProfile::from_name(&v).is_none() {
                    return Err(CliError::Invalid(name, v));
                }
                o.bot = Some(v);
            }
            "--headless" => o.headless = true,
            "--autoplay" => o.autoplay = true,
            "--help" | "-h" => o.help = true,
            "--version" | "-V" => o.version = true,
            _ => return Err(CliError::Unknown(arg)),
        }
    }
    Ok(o)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> Result<Options, CliError> {
        parse(s.split_whitespace().map(String::from))
    }

    #[test]
    fn parses_flags() {
        let o = p("--lang ru --difficulty veteran --seed 7 --headless --seeds 5").unwrap();
        assert_eq!(o.lang, Some(Lang::Ru));
        assert_eq!(o.difficulty, Some(Difficulty::Veteran));
        assert_eq!(o.seed, Some(7));
        assert!(o.headless);
        assert_eq!(o.seeds, Some(5));
        let o = p("--lang=uz --autoplay").unwrap();
        assert_eq!(o.lang, Some(Lang::Uz));
        assert!(o.autoplay);
    }

    #[test]
    fn rejects_bad_input() {
        assert!(matches!(p("--frobnicate"), Err(CliError::Unknown(_))));
        assert!(matches!(p("--lang xx"), Err(CliError::Invalid(..))));
        assert!(matches!(p("--seed"), Err(CliError::Missing(_))));
        assert!(matches!(p("--seeds 0"), Err(CliError::Invalid(..))));
    }
}
