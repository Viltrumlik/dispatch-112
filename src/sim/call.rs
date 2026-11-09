//! A live emergency call.

use super::catalog::{self, Needs, Template};
use super::map::{District, Pos};
use super::units::UnitKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CallStatus {
    /// In the queue (with or without units assigned).
    Waiting,
    /// Dismissed by the dispatcher. A real call dismissed this way calls back later.
    Dismissed,
    Resolved,
    /// Help came too late.
    Failed,
    /// A prank caller gave up and hung up.
    Expired,
    /// A crew arrived and found nothing: the prank is exposed.
    PrankExposed,
    /// Still open at 06:00; handed to the morning shift.
    HandedOver,
}

/// What the last crew on scene radioed back.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Report {
    /// Nothing there: a prank.
    Nothing,
    /// Wrong service; the true needs are now known.
    WrongUnit,
    /// The right service, but other services are needed too.
    NeedMore,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Call {
    /// Display number (#1, #2...), unique within a shift.
    pub id: u32,
    pub template: usize,
    pub variant: u8,
    pub pos: Pos,
    pub district: District,
    pub arrived: u32,
    pub severity: u8,
    pub base_severity: u8,
    pub needs: Needs,
    /// Units of each kind that have completed their work.
    pub served: Needs,
    pub prank: bool,
    pub arson: bool,
    pub finale: bool,
    pub status: CallStatus,
    pub next_escalation: u32,
    /// Severity-3 calls fail at this tick unless covered.
    pub deadline: Option<u32>,
    pub first_response: Option<u32>,
    pub flagged: bool,
    /// A crew has been on scene: the true needs are known to the dispatcher.
    pub revealed: bool,
    pub report: Option<Report>,
    /// Re-opened after a wrongful dismissal.
    pub callback: bool,
    pub callback_at: Option<u32>,
    pub expire_at: Option<u32>,
    pub closed_at: Option<u32>,
    /// A dispatch to a prank has already been penalised.
    pub prank_penalised: bool,
}

impl Call {
    pub fn template(&self) -> &'static Template {
        catalog::get(self.template)
    }

    pub fn text_key(&self) -> String {
        self.template().text_key(self.variant)
    }

    pub fn is_open(&self) -> bool {
        self.status == CallStatus::Waiting
    }

    /// What the dispatcher can reasonably believe this call needs.
    pub fn known_needs(&self) -> Needs {
        if self.revealed {
            self.needs
        } else {
            self.template().claim
        }
    }

    /// Kinds still missing after counting finished work (not counting units on the way).
    pub fn remaining(&self, kind: UnitKind) -> u8 {
        self.needs.get(kind).saturating_sub(self.served.get(kind))
    }

    pub fn fully_served(&self) -> bool {
        UnitKind::ALL.iter().all(|&k| self.served.get(k) >= self.needs.get(k))
    }
}
