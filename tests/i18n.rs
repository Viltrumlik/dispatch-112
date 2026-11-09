//! Translation completeness: identical keys, matching placeholders, no empty
//! strings, and every key the game asks for exists.

use dispatch112::i18n::{parse, placeholders, t, Lang};
use dispatch112::sim::catalog::{hint_ids, TEMPLATES};
use dispatch112::sim::shift::{CmdError, Tip};
use dispatch112::sim::*;
use std::collections::BTreeSet;

fn keys(l: Lang) -> BTreeSet<String> {
    parse(l).keys().cloned().collect()
}

#[test]
fn all_languages_have_identical_keys() {
    let en = keys(Lang::En);
    for l in [Lang::Uz, Lang::Ru] {
        let k = keys(l);
        let missing: Vec<_> = en.difference(&k).collect();
        let extra: Vec<_> = k.difference(&en).collect();
        assert!(missing.is_empty() && extra.is_empty(), "{l:?}: missing {missing:?}, extra {extra:?}");
    }
}

#[test]
fn placeholders_match_and_nothing_is_empty() {
    let en = parse(Lang::En);
    for l in Lang::ALL {
        let m = parse(l);
        for (k, v) in &m {
            assert!(!v.trim().is_empty(), "{l:?} {k} is empty");
            assert_eq!(placeholders(v), placeholders(&en[k]), "{l:?} {k}: placeholders differ");
        }
    }
}

#[test]
fn uzbek_uses_ascii_apostrophes_only() {
    for (k, v) in parse(Lang::Uz) {
        assert!(!v.contains(['ʻ', 'ʼ', '‘', '’', '`']), "uz {k} uses a non-ASCII apostrophe");
        if !k.starts_with("cli.") && !k.starts_with("help.") {
            assert!(!v.chars().any(|c| ('\u{0400}'..='\u{04FF}').contains(&c)), "uz {k} contains Cyrillic");
        }
    }
}

#[test]
fn arsonist_signature_phrase_is_identical_in_each_language() {
    let ids = [
        "arson_warehouse.0",
        "arson_warehouse.1",
        "arson_kiosk.0",
        "arson_kiosk.1",
        "arson_garage.0",
        "arson_garage.1",
        "arson_finale.0",
        "botir_petrol.0",
    ];
    let phrase = |l: Lang| match l {
        Lang::En => "Hurry — or you'll be late again",
        Lang::Ru => "Поторопитесь — а то опять опоздаете",
        Lang::Uz => "Shoshiling — bo'lmasa yana kech qolasiz",
    };
    for l in Lang::ALL {
        for id in ids {
            let text = t(l, &format!("call.{id}"));
            assert!(text.contains(phrase(l)), "{l:?} call.{id} lacks the phrase: {text}");
        }
    }
}

#[test]
fn every_key_the_game_uses_exists() {
    let mut need: Vec<String> = Vec::new();
    for tpl in TEMPLATES {
        for v in 0..tpl.variants {
            need.push(tpl.text_key(v));
        }
    }
    need.extend(hint_ids().into_iter().map(|h| format!("hint.{h}")));
    need.extend(District::ALL.iter().map(|d| d.key().to_string()));
    need.extend(UnitKind::ALL.iter().map(|k| k.key().to_string()));
    need.extend(Difficulty::ALL.iter().flat_map(|d| [d.key().to_string(), format!("title.desc.{}", d.code())]));
    for u in UpgradeKind::ALL {
        need.push(u.key().to_string());
        need.push(format!("{}.desc", u.key()));
    }
    for e in [Ending::GoldenHeadset, Ending::SteadyHand, Ending::NightDetective, Ending::BadgeReturned, Ending::Burnout]
    {
        need.push(e.key().to_string());
        need.push(format!("{}.text", e.key()));
    }
    for tip in
        [Tip::Welcome, Tip::FirstCall, Tip::Prank, Tip::Multi, Tip::Escalation, Tip::Report, Tip::Arson, Tip::Stress]
    {
        need.push(tip.key().to_string());
    }
    for e in [
        CmdError::ShiftOver,
        CmdError::NoSuchCall,
        CmdError::CallClosed,
        CmdError::NoUnits,
        CmdError::NoSuchUnit(0),
        CmdError::UnitBusy(0),
        CmdError::Unreachable,
        CmdError::CoffeeUsed,
        CmdError::NotRecallable,
    ] {
        need.push(e.key().to_string());
    }
    for n in 0..5 {
        need.push(format!("night.{n}"));
        need.push(format!("night.event.{n}"));
        need.push(format!("briefing.{n}"));
    }
    for g in ["S", "A", "B", "C", "D"] {
        need.push(format!("zuhra.{g}"));
    }
    for s in 1..=3 {
        need.push(format!("ui.sev.{s}"));
    }
    for l in Lang::ALL {
        let m = parse(l);
        for k in &need {
            assert!(m.contains_key(k), "{l:?} is missing {k}");
        }
    }
}

#[test]
fn call_texts_fit_the_details_pane() {
    // The details pane is ~58 columns wide inside and shows about four lines of text.
    for l in Lang::ALL {
        for tpl in TEMPLATES {
            for v in 0..tpl.variants {
                let n = t(l, &tpl.text_key(v)).chars().count();
                assert!(n <= 240, "{l:?} {} is {n} characters long", tpl.text_key(v));
            }
        }
    }
}
