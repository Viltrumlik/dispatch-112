//! Navbatchi 112 / Диспетчер 112 / Dispatch 112 — a real-time emergency-dispatch
//! simulator for the terminal.
//!
//! * [`sim`] — the pure, deterministic simulation (no terminal I/O).
//! * [`bot`] — a scripted dispatcher for autoplay, headless runs and tests.
//! * [`i18n`] — embedded Uzbek / Russian / English text.
//! * [`ui`] — the ratatui front end.

pub mod bot;
pub mod cli;
pub mod headless;
pub mod i18n;
pub mod save;
pub mod sim;
pub mod ui;
