//! Embedded translations (Uzbek, Russian, English). Each language has two flat
//! JSON files — `ui.json` (interface) and `story.json` (calls and story) — that
//! are merged into one table. All three languages have identical keys
//! (checked by `tests/i18n.rs`).

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Lang {
    Uz,
    Ru,
    En,
}

impl Lang {
    pub const ALL: [Lang; 3] = [Lang::Uz, Lang::Ru, Lang::En];

    pub fn code(self) -> &'static str {
        match self {
            Lang::Uz => "uz",
            Lang::Ru => "ru",
            Lang::En => "en",
        }
    }

    pub fn from_code(s: &str) -> Option<Lang> {
        let s = s.to_ascii_lowercase();
        if s.starts_with("uz") {
            Some(Lang::Uz)
        } else if s.starts_with("ru") {
            Some(Lang::Ru)
        } else if s.starts_with("en") {
            Some(Lang::En)
        } else {
            None
        }
    }

    pub fn next(self) -> Lang {
        match self {
            Lang::Uz => Lang::Ru,
            Lang::Ru => Lang::En,
            Lang::En => Lang::Uz,
        }
    }

    /// The language's own name.
    pub fn native_name(self) -> &'static str {
        match self {
            Lang::Uz => "O'zbekcha",
            Lang::Ru => "Русский",
            Lang::En => "English",
        }
    }

    /// Guess from the environment (LC_ALL / LC_MESSAGES / LANG); Uzbek by default.
    pub fn detect() -> Lang {
        for var in ["LC_ALL", "LC_MESSAGES", "LANG"] {
            if let Ok(v) = std::env::var(var) {
                if let Some(l) = Lang::from_code(&v) {
                    return l;
                }
            }
        }
        Lang::Uz
    }
}

/// Raw embedded JSON sources for a language: (ui, story).
pub fn sources(lang: Lang) -> [&'static str; 2] {
    match lang {
        Lang::Uz => [include_str!("uz/ui.json"), include_str!("uz/story.json")],
        Lang::Ru => [include_str!("ru/ui.json"), include_str!("ru/story.json")],
        Lang::En => [include_str!("en/ui.json"), include_str!("en/story.json")],
    }
}

/// Parse and merge the sources of a language. Panics on malformed JSON or a
/// key defined in both files (both are build-time programming errors).
pub fn parse(lang: Lang) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for src in sources(lang) {
        let m: HashMap<String, String> =
            serde_json::from_str(src).unwrap_or_else(|e| panic!("bad {} translation JSON: {e}", lang.code()));
        for (k, v) in m {
            assert!(out.insert(k.clone(), v).is_none(), "duplicate key {k} in {}", lang.code());
        }
    }
    out
}

fn table(lang: Lang) -> &'static HashMap<String, String> {
    static T: [OnceLock<HashMap<String, String>>; 3] = [OnceLock::new(), OnceLock::new(), OnceLock::new()];
    let i = match lang {
        Lang::Uz => 0,
        Lang::Ru => 1,
        Lang::En => 2,
    };
    T[i].get_or_init(|| parse(lang))
}

/// Translate a key. Falls back to English, then to the key marker.
pub fn t(lang: Lang, key: &str) -> &'static str {
    table(lang).get(key).or_else(|| table(Lang::En).get(key)).map(|s| s.as_str()).unwrap_or("[?]")
}

/// Translate and substitute `{name}` placeholders.
pub fn tf(lang: Lang, key: &str, args: &[(&str, String)]) -> String {
    let mut s = t(lang, key).to_string();
    for (name, value) in args {
        s = s.replace(&format!("{{{name}}}"), value);
    }
    s
}

/// The set of `{placeholder}` names in a string.
pub fn placeholders(s: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let mut rest = s;
    while let Some(start) = rest.find('{') {
        let after = &rest[start + 1..];
        match after.find('}') {
            Some(end) => {
                let name = &after[..end];
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
                    out.insert(name.to_string());
                }
                rest = &after[end + 1..];
            }
            None => break,
        }
    }
    out
}

/// Shorthand for building placeholder arguments.
#[macro_export]
macro_rules! args {
    ($($k:ident = $v:expr),* $(,)?) => {
        &[$((stringify!($k), $v.to_string())),*]
    };
}
