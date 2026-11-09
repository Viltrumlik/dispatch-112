//! Save/load round trip in a temporary directory, and the DISPATCH112_HOME override.

use dispatch112::bot::{play_shift, Bot, BotProfile};
use dispatch112::i18n::Lang;
use dispatch112::save::{self, SaveData};
use dispatch112::sim::*;

fn temp_dir(name: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("dispatch112-save-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    d
}

#[test]
fn round_trip_between_shifts() {
    let dir = temp_dir("rt");
    assert!(save::load(&dir).is_none());
    let mut week = Week::new(Difficulty::Veteran, 31);
    let mut s = week.start_shift();
    play_shift(&mut s, &mut Bot::new(BotProfile::good(), 2));
    week.finish_shift(&s);
    week.apply_upgrade(week.upgrade_options()[0]);
    let data = SaveData::new(Lang::Ru, week.clone());
    save::save(&dir, &data).unwrap();
    let back = save::load(&dir).expect("saved game loads");
    assert_eq!(back, data);
    assert_eq!(back.week.night, 1);
    // The next shift starts identically from the loaded state.
    let a = week.start_shift();
    let b = back.week.start_shift();
    assert_eq!(a.units.len(), b.units.len());
    assert_eq!(a.trust, b.trust);
    save::clear(&dir).unwrap();
    assert!(save::load(&dir).is_none());
    save::clear(&dir).unwrap();
}

#[test]
fn finished_or_corrupt_saves_are_ignored() {
    let dir = temp_dir("bad");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(save::path(&dir), "{ not json").unwrap();
    assert!(save::load(&dir).is_none());
    let mut week = Week::new(Difficulty::Rookie, 1);
    week.ending = Some(Ending::SteadyHand);
    save::save(&dir, &SaveData::new(Lang::En, week)).unwrap();
    assert!(save::load(&dir).is_none(), "a finished week is not offered as Continue");
}

#[test]
fn env_var_overrides_the_save_directory() {
    let dir = temp_dir("env");
    std::env::set_var("DISPATCH112_HOME", &dir);
    assert_eq!(save::default_dir(), dir);
    std::env::remove_var("DISPATCH112_HOME");
    assert!(save::default_dir().ends_with(".dispatch112"));
}
