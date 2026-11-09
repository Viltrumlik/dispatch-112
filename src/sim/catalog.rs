//! Call templates. Each template has a hidden truth (`needs`), what the caller's
//! words suggest at face value (`claim`), severity range, on-scene work time,
//! scheduling weight and placement. Texts live in the i18n files under
//! `call.<id>.<variant>`; screening hints under `hint.<hint>`.

use super::map::District;
use super::units::UnitKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash, Serialize, Deserialize)]
pub struct Needs {
    pub amb: u8,
    pub fire: u8,
    pub police: u8,
}

impl Needs {
    pub const NONE: Needs = Needs { amb: 0, fire: 0, police: 0 };

    pub const fn new(amb: u8, fire: u8, police: u8) -> Needs {
        Needs { amb, fire, police }
    }

    pub fn get(&self, k: UnitKind) -> u8 {
        match k {
            UnitKind::Ambulance => self.amb,
            UnitKind::Fire => self.fire,
            UnitKind::Police => self.police,
        }
    }

    pub fn get_mut(&mut self, k: UnitKind) -> &mut u8 {
        match k {
            UnitKind::Ambulance => &mut self.amb,
            UnitKind::Fire => &mut self.fire,
            UnitKind::Police => &mut self.police,
        }
    }

    pub fn total(&self) -> u8 {
        self.amb + self.fire + self.police
    }

    pub fn is_empty(&self) -> bool {
        self.total() == 0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedPlace {
    KarimHome,
    BotirBakery,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Place {
    /// Anywhere people live (not the Industrial Zone).
    Residential,
    /// Any district.
    Any,
    In(District),
    NotIn(District),
    Fixed(FixedPlace),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TKind {
    Normal,
    Storm,
    Stadium,
    Decoy,
    Scripted,
    Arson,
    Finale,
    FinaleDecoy,
}

#[derive(Clone, Copy, Debug)]
pub struct Template {
    pub id: &'static str,
    pub variants: u8,
    pub needs: Needs,
    pub claim: Needs,
    pub prank: bool,
    pub sev_min: u8,
    pub sev_max: u8,
    /// Base on-scene work time in ticks.
    pub work: u32,
    /// Relative weight for the random scheduler (0 = scripted only).
    pub weight: u32,
    pub place: Place,
    pub kind: TKind,
    pub hint: Option<&'static str>,
    /// Part of the arsonist pattern: the correct thing is to flag it.
    pub arson: bool,
}

impl Template {
    const fn new(id: &'static str, needs: Needs, sev: (u8, u8), work: u32, weight: u32) -> Template {
        Template {
            id,
            variants: 2,
            needs,
            claim: needs,
            prank: false,
            sev_min: sev.0,
            sev_max: sev.1,
            work,
            weight,
            place: Place::Residential,
            kind: TKind::Normal,
            hint: None,
            arson: false,
        }
    }
    const fn prank(id: &'static str, claim: Needs, sev: u8, weight: u32, hint: &'static str) -> Template {
        let mut t = Template::new(id, Needs::NONE, (sev, sev), 2, weight);
        t.claim = claim;
        t.prank = true;
        t.hint = Some(hint);
        t
    }
    const fn claim(mut self, c: Needs) -> Self {
        self.claim = c;
        self
    }
    const fn hint(mut self, h: &'static str) -> Self {
        self.hint = Some(h);
        self
    }
    const fn at(mut self, p: Place) -> Self {
        self.place = p;
        self
    }
    const fn kind(mut self, k: TKind) -> Self {
        self.kind = k;
        self
    }
    const fn variants(mut self, n: u8) -> Self {
        self.variants = n;
        self
    }
    const fn arson(mut self) -> Self {
        self.arson = true;
        self
    }

    /// The caller's words suggest something other than the truth.
    pub fn is_misleading(&self) -> bool {
        !self.prank && self.claim != self.needs
    }

    pub fn is_multi(&self) -> bool {
        self.needs.total() > 1
    }

    pub fn text_key(&self, variant: u8) -> String {
        format!("call.{}.{}", self.id, variant)
    }
}

const A: Needs = Needs::new(1, 0, 0);
const F: Needs = Needs::new(0, 1, 0);
const P: Needs = Needs::new(0, 0, 1);
const AF: Needs = Needs::new(1, 1, 0);
const AP: Needs = Needs::new(1, 0, 1);
const FP: Needs = Needs::new(0, 1, 1);
const ANY: Place = Place::Any;
const IND: District = District::Industrial;

pub static TEMPLATES: &[Template] = &[
    // --- Medical ---
    Template::new("heart_attack", A, (2, 3), 16, 10),
    Template::new("stroke", A, (3, 3), 16, 5),
    Template::new("childbirth", A, (2, 2), 18, 5),
    Template::new("child_fever", A, (1, 2), 12, 7),
    Template::new("diabetic", A, (2, 2), 12, 5),
    Template::new("allergy", A, (2, 3), 12, 5),
    Template::new("elderly_fall", A, (1, 2), 14, 8),
    Template::new("drowning", A, (3, 3), 14, 3).at(ANY),
    Template::new("wedding_poisoning", Needs::new(2, 0, 0), (2, 2), 16, 3),
    Template::new("asthma", A, (2, 3), 12, 5),
    Template::new("unconscious_party", A, (3, 3), 14, 3),
    Template::new("kitchen_burn", A, (1, 1), 10, 4),
    // --- Fire ---
    Template::new("apartment_fire", F, (2, 3), 22, 6),
    Template::new("kitchen_fire", F, (1, 2), 14, 6),
    Template::new("car_fire", F, (1, 2), 12, 5).at(ANY),
    Template::new("trash_fire", F, (1, 1), 8, 5).at(ANY),
    Template::new("gas_leak", F, (2, 2), 16, 5),
    Template::new("bazaar_fire", Needs::new(0, 2, 0), (2, 3), 24, 2).at(Place::In(District::Bazaar)),
    Template::new("tandir_roof", F, (1, 2), 14, 3).at(Place::In(District::OldTown)),
    Template::new("decoy_transformer", F, (2, 2), 16, 3).at(Place::In(IND)).kind(TKind::Decoy),
    Template::new("decoy_petrol_station", F, (1, 1), 12, 3).at(Place::NotIn(IND)).kind(TKind::Decoy),
    Template::new("decoy_garage_heater", F, (2, 2), 14, 3).at(Place::NotIn(IND)).kind(TKind::Decoy),
    // --- Police ---
    Template::new("burglary", P, (2, 2), 14, 7),
    Template::new("street_fight", P, (2, 2), 12, 7).at(ANY),
    Template::new("domestic", P, (2, 3), 14, 6),
    Template::new("car_theft", P, (1, 1), 10, 5),
    Template::new("drunk_driver", P, (2, 2), 10, 5).at(ANY),
    Template::new("armed_robbery", Needs::new(0, 0, 2), (3, 3), 16, 2),
    Template::new("missing_child", P, (2, 3), 18, 3),
    Template::new("loud_party", P, (1, 1), 8, 5),
    Template::new("vandalism", P, (1, 1), 8, 3).at(ANY),
    // --- Several services ---
    Template::new("car_crash", AP, (2, 3), 16, 6).at(ANY),
    Template::new("crash_fire", AF, (3, 3), 18, 3).at(ANY),
    Template::new("stabbing", AP, (3, 3), 14, 3).at(ANY),
    Template::new("gas_explosion", Needs::new(1, 2, 0), (3, 3), 24, 1),
    Template::new("hit_and_run", AP, (2, 3), 14, 4).at(ANY),
    // --- Misleading: the words point one way, the truth another ---
    Template::new("smoke_grandpa", AF, (2, 3), 16, 4).claim(A).hint("smoke"),
    Template::new("co_poisoning", AF, (2, 3), 16, 3).claim(A).hint("co"),
    Template::new("scream_noise", AP, (2, 2), 14, 3).claim(P).hint("scream"),
    Template::new("engine_running", A, (2, 2), 12, 3).claim(P).hint("unconscious"),
    Template::new("balcony_child", F, (3, 3), 12, 2).claim(P).hint("ladder"),
    // --- Wednesday storm ---
    Template::new("tree_on_car", AF, (2, 3), 18, 6).at(ANY).kind(TKind::Storm),
    Template::new("power_line", F, (2, 2), 14, 6).at(ANY).kind(TKind::Storm),
    Template::new("flooded_underpass", AF, (3, 3), 18, 3).at(ANY).kind(TKind::Storm),
    Template::new("roof_sheet", A, (2, 2), 12, 4).at(ANY).kind(TKind::Storm),
    // --- Thursday stadium event ---
    Template::new("fan_brawl", Needs::new(0, 0, 2), (2, 2), 14, 6)
        .at(Place::In(District::Stadium))
        .kind(TKind::Stadium),
    Template::new("crowd_crush", Needs::new(2, 0, 1), (3, 3), 18, 2)
        .at(Place::In(District::Stadium))
        .kind(TKind::Stadium),
    Template::new("pickpocket", P, (1, 1), 8, 4).at(Place::In(District::Stadium)).kind(TKind::Stadium),
    Template::new("flare_smoke", AP, (2, 2), 14, 3)
        .claim(F)
        .hint("flare")
        .at(Place::In(District::Stadium))
        .kind(TKind::Stadium),
    // --- Pranks and non-emergencies (dismiss with D) ---
    Template::prank("prank_ufo", P, 2, 3, "laugh"),
    Template::prank("prank_lion", P, 3, 2, "zoo"),
    Template::prank("prank_pizza", Needs::NONE, 1, 3, "nonemergency"),
    Template::prank("prank_dare", F, 2, 3, "laugh"),
    Template::prank("prank_shashlik", F, 2, 3, "shashlik"),
    Template::prank("prank_cat", F, 1, 2, "nonemergency"),
    Template::prank("prank_wifi", P, 1, 2, "nonemergency"),
    // --- Recurring callers (scripted) ---
    Template::prank("karim_lonely", Needs::NONE, 1, 0, "karim")
        .variants(3)
        .at(Place::Fixed(FixedPlace::KarimHome))
        .kind(TKind::Scripted),
    Template::new("karim_heart", A, (3, 3), 16, 0)
        .claim(Needs::NONE)
        .hint("karim_heart")
        .variants(1)
        .at(Place::Fixed(FixedPlace::KarimHome))
        .kind(TKind::Scripted),
    Template::prank("jasur_prank", F, 2, 0, "jasur").variants(3).kind(TKind::Scripted),
    Template::new("jasur_real", F, (2, 2), 16, 0).hint("jasur_real").variants(1).kind(TKind::Scripted),
    Template::new("botir_gas", F, (2, 2), 14, 0)
        .variants(1)
        .at(Place::Fixed(FixedPlace::BotirBakery))
        .kind(TKind::Scripted),
    Template::new("botir_breakin", P, (2, 2), 12, 0)
        .variants(1)
        .at(Place::Fixed(FixedPlace::BotirBakery))
        .kind(TKind::Scripted),
    Template::new("botir_van", AF, (2, 2), 16, 0).variants(1).at(ANY).kind(TKind::Scripted),
    Template::new("botir_petrol", P, (1, 1), 10, 0)
        .variants(1)
        .hint("petrol")
        .at(Place::In(IND))
        .kind(TKind::Scripted)
        .arson(),
    // --- The arsonist ---
    Template::new("arson_warehouse", F, (2, 2), 20, 0).hint("petrol").at(Place::In(IND)).kind(TKind::Arson).arson(),
    Template::new("arson_kiosk", F, (2, 2), 16, 0).hint("petrol").at(Place::In(IND)).kind(TKind::Arson).arson(),
    Template::new("arson_garage", F, (2, 2), 18, 0).hint("petrol").at(Place::In(IND)).kind(TKind::Arson).arson(),
    Template::new("arson_finale", FP, (2, 2), 20, 0)
        .claim(F)
        .hint("finale")
        .variants(1)
        .at(Place::In(IND))
        .kind(TKind::Finale)
        .arson(),
    Template::new("finale_decoy", F, (2, 3), 18, 0)
        .variants(1)
        .at(Place::In(District::Bazaar))
        .kind(TKind::FinaleDecoy),
];

/// Index of a template by id. Panics on an unknown id (a programming error).
pub fn find(id: &str) -> usize {
    TEMPLATES.iter().position(|t| t.id == id).unwrap_or_else(|| panic!("unknown call template {id}"))
}

pub fn get(i: usize) -> &'static Template {
    &TEMPLATES[i]
}

/// All screening hint ids used by templates.
pub fn hint_ids() -> Vec<&'static str> {
    let mut v: Vec<&'static str> = TEMPLATES.iter().filter_map(|t| t.hint).collect();
    v.sort_unstable();
    v.dedup();
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn at_least_forty_templates_with_unique_ids() {
        assert!(TEMPLATES.len() >= 40);
        let mut ids: Vec<_> = TEMPLATES.iter().map(|t| t.id).collect();
        ids.sort_unstable();
        let n = ids.len();
        ids.dedup();
        assert_eq!(n, ids.len(), "duplicate template ids");
    }

    #[test]
    fn templates_are_consistent() {
        for t in TEMPLATES {
            assert!(t.variants >= 1);
            assert!(t.sev_min >= 1 && t.sev_max <= 3 && t.sev_min <= t.sev_max, "{}", t.id);
            if t.prank {
                assert!(t.needs.is_empty(), "{}", t.id);
            } else {
                assert!(!t.needs.is_empty(), "{}", t.id);
                assert!(t.work > 0);
            }
        }
        assert!(TEMPLATES.iter().filter(|t| t.prank).count() >= 6);
        assert!(TEMPLATES.iter().filter(|t| t.is_misleading()).count() >= 5);
        assert!(TEMPLATES.iter().filter(|t| t.is_multi()).count() >= 8);
    }
}
