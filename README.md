# Navbatchi 112 / Диспетчер 112 / Dispatch 112

![Rust: 2021](https://img.shields.io/badge/Rust-2021-000000?logo=rust&logoColor=white) ![ratatui: 0.30](https://img.shields.io/badge/ratatui-0.30-6E40C9) ![tests: 55](https://img.shields.io/badge/tests-55-2ea44f) ![languages: uz · ru · en](https://img.shields.io/badge/languages-uz%20%C2%B7%20ru%20%C2%B7%20en-orange) ![license: proprietary](https://img.shields.io/badge/license-proprietary-lightgrey)

A real-time emergency-dispatch simulator for the terminal. Calls scroll in, the ASCII map of the city of Yangiobod is
alive with ambulances, fire engines and patrol cars, and you have minutes — game minutes, which pass quickly — to read
each call, work out what is really going on, and send the right units, or dismiss it as a prank. Five night shifts,
Monday to Friday, 22:00 to 06:00. Somewhere in the fire calls, a quiet pattern is hiding.

- Real-time dispatching on a 58×18 box-drawn city map: roads, a river with four bridges, six districts, a hospital,
  a fire station and a police station; units drive along the roads and you watch them go.
- 69 call templates with variants: heart attacks, kitchen fires, burglaries, crashes that need two services, misleading
  calls ("grandpa fell... oh, and there's smoke"), pranks, non-emergencies, storm and stadium incidents.
- Severity escalates while callers wait; critical calls are lost if nobody arrives in time. Crews radio back what they
  find, so a wrong guess costs time, not the game.
- City trust and stress meters, a once-per-shift coffee break, letter grades S–D per shift, an upgrade between shifts.
- A story thread: flag the arsonist's calls for Inspector Dilshod, set a trap, and catch him on Friday. Five endings.
- Fully playable in Uzbek (Latin), Russian and English; switch at any time with `L`.
- Three difficulties, a deterministic seeded simulation, a demo autoplay mode and a headless balance runner.

## What it looks like

Monday night, about an hour into the shift, captured from the built-in demo (`--autoplay --lang en`). Calls on
the left, the city map in the middle (`H` hospital, `F` fire station, `P` police, digits are moving units, `!` is
a waiting call), the fleet on the right, the selected call and the radio log below. In a real terminal it is in
colour.

```text
 Monday 23:07 │ Trust ██████░░  63 │ Stress ░░░░░░░░   0 │ ×4 │ Coffee: C │ DEMO
╭ Calls (2) ──────────────────╮╭ Yangiobod ───────────────────────────────────────────────╮╭ Units ────────────────────╮
│▶#3  !   18m●Bazaar          ││┌──┬───┬─────────!───┬──────┬──────┬──────┬──────┬───────┐││ 1 A1 returning ~2m        │
│ #5  !!   2m Old Town        │││··│···│·······♣♣♣♣♣♣│······│······│······│······│·······│││ 2 A2 at base ~5m          │
│                             ││├──┼───┤·······♣♣♣♣♣♣│······│······│······│······│·······│││ 3 A3 at base ~5m          │
│                             │││··│···│·······♣♣♣♣♣♣1······│······│······│······│·······│││ 4 F1 returning ~10m       │
│                             ││├──┴───┼──────H──────┼─6────┼──────┼──────P──────┼───────┤││ 5 F2 at base ~11m         │
│                             │││······│······│······│······│······│······│······│·······│││ 6 P1 at #3                │
│                             │││······│······│······│······│······│······│······│·······│││ 7 P2 at base ~8m          │
│                             ││└──────┼──────┴──────┼──────┴──────┼──────┴──────┼───────┘││                           │
│                             ││≈≈≈≈≈≈≈║≈≈≈≈≈≈≈≈≈≈≈≈≈║≈≈≈≈≈≈≈≈≈≈≈≈≈║≈≈≈≈≈≈≈≈≈≈≈≈≈║≈≈≈≈≈≈≈≈││A ambulance                │
│                             ││┌──────┼──────┬──────┼──────┬─4────F──────┬──────┼───────┐││F fire engine              │
│                             │││······│······│······│······│▪▪▪▪▪▪│▪▪▪▪▪▪│♣♣♣♣♣♣│░░░░░░░│││P police                   │
│                             │││······│······│······├──────┼──────┤▪▪▪▪▪▪│♣♣♣♣♣♣│░░░░░░░│││                           │
│                             │││······│······│······│······│······│▪▪▪▪▪▪│♣♣♣♣♣♣│░░░░░░░│││                           │
│                             ││├──────┼──────┼──────┼──────┴──┬───┼──────┼──────┤░░░░░░░│││                           │
│                             │││······│······│······│▪▪▪▪▪▪▪▪▪│▪▪▪│▪▪▪▪▪▪│······│░░░░░░░│││                           │
│                             │││══════│══════│══════│▪▪▪▪▪▪▪▪▪│▪▪▪│▪▪▪▪▪▪├──────┤░░░░░░░│││                           │
│                             │││······│······│······│▪▪▪▪▪▪▪▪▪│▪▪▪│▪▪▪▪▪▪│······│░░░░░░░│││                           │
│                             ││└──────┴──────┴──────┴─────────┴───┴──────┴──────┴───────┘││                           │
╰─────────────────────────────╯╰ H hospital F fire station P police ──────────────────────╯╰───────────────────────────╯
╭ Call #3 — Bazaar ──────────────────────────────────────────────────────╮╭ Radio ─────────────────────────────────────╮
│!   minor · waiting 18 min                                              ││22:49 New call #3 — Bazaar                  │
│“Someone is taking the wheels off my car right now — I can see them from││22:52 New call #4 — New Blocks              │
│the window.”                                                            ││22:56 #4 dismissed                          │
│Units: P1 on scene                                                      ││22:57 #2 resolved                           │
│Pick units with 1–9, then Enter to send. D dismisses, F flags.          ││22:59 P1 on scene at #3                     │
│                                                                        ││23:05 New call #5 — Old Town                │
╰────────────────────────────────────────────────────────────────────────╯╰────────────────────────────────────────────╯
 ↑↓ call  1-9 unit  Enter send  D dismiss  F flag  R recall  C coffee  Spc pause  ? help  Q quit
```

## Qisqacha · Кратко

**🇺🇿 O'zbekcha.** Terminal uchun real vaqtli tez yordam dispetcheri simulyatori. Yangiobod shahri, dushanbadan jumagacha besh tungi smena. Qo'ng'iroqlarni o'qing, aslida nima bo'lganini aniqlang va kerakli xizmatni (tez yordam, o't o'chiruvchilar yoki politsiya) jo'nating, yolg'on chaqiruvlarni rad eting. Yong'in chaqiruvlari orasida o't qo'yuvchining izi yashiringan. 5 ta yakun, 3 qiyinlik darajasi. O'zbek tilida: `./target/release/dispatch112 --lang uz`.

**🇷🇺 Русский.** Симулятор диспетчера службы 112 в реальном времени для терминала. Город Янгиобод, пять ночных смен с понедельника по пятницу. Читайте звонки, понимайте, что случилось на самом деле, отправляйте скорую, пожарных или полицию и отсеивайте ложные вызовы. Среди пожаров прячется почерк поджигателя. 5 концовок, 3 уровня сложности. На русском: `./target/release/dispatch112 --lang ru`.

## Story

You are a rookie at the unified 112 centre of Yangiobod. Zuhra-opa, who has answered these phones for thirty years,
sits next to you for one last week — Friday is her final shift. Karim-bobo calls every night because he is lonely.
Jasur, fourteen, thinks prank calls are hilarious. Botir the baker works nights and sees everything. And every night,
somewhere in the Industrial Zone, a warehouse, a kiosk or a garage catches fire — it always smells of petrol, and the
same calm voice always ends the call the same way.

## Controls

| Key | Action |
|---|---|
| `↑` / `↓` | select a call (or a unit after `Tab`) |
| `Tab` | switch between the call queue and the unit list |
| `1`–`9` | pick / unpick units to send (several are allowed) |
| `Enter` | send the picked units to the selected call |
| `Esc` | clear the picked units |
| `D` | dismiss the selected call (pranks, non-emergencies) |
| `F` | flag the selected call as suspicious for Inspector Dilshod |
| `R` | recall the unit under the cursor, or every unit of the selected call |
| `C` | coffee break (once per shift) |
| `Space` | pause / resume |
| `+` / `-` | speed ×1 / ×2 / ×4 |
| `L` | language: O'zbekcha / Русский / English |
| `?` | help |
| `Q` | quit (asks first) |

Unit letters on screen are the same in every language: **A** ambulance, **F** fire engine, **P** patrol car. On the map,
moving units are shown by their key number, calls by `!` (yellow → red by severity).

## Requirements

- Rust stable (built and tested with 1.99, edition 2021) and Cargo.
- A terminal of at least **100×30** characters with UTF-8 and colour (macOS Terminal, iTerm2, any modern Linux terminal,
  Windows Terminal). A smaller window shows a "please enlarge the terminal" message.

## Build and run

```sh
git clone https://github.com/Viltrumlik/dispatch-112.git
cd dispatch-112
cargo build --release

# play (language and difficulty can also be chosen on the title screen)
./target/release/dispatch112
./target/release/dispatch112 --lang uz --difficulty rookie
./target/release/dispatch112 --lang ru --difficulty veteran --seed 2026

# watch the bot play inside the TUI (does not touch your save)
./target/release/dispatch112 --autoplay --lang en

# all options
./target/release/dispatch112 --help
```

Progress is saved after every finished shift to `~/.dispatch112/progress.json`; choose **Continue** on the title
screen to resume. Set `DISPATCH112_HOME` to use another folder.

## Tests and the headless balance run

```sh
cargo test                       # 55 tests: engine, whole-week simulations, i18n, save, UI rendering
cargo clippy --all-targets -- -D warnings

# one week played by the bot, printed as a table
./target/release/dispatch112 --headless --seed 42 --difficulty dispatcher --lang en

# balance table: seeds 1..50 for every difficulty (add --difficulty to run just one)
./target/release/dispatch112 --headless --seeds 50 --lang en
./target/release/dispatch112 --headless --seeds 50 --bot sloppy --lang en
```

Bot profiles: `good` (skilled and attentive: human-like reaction time, takes ~25 % of misleading calls at face value,
spots ~80 % of pranks, flags the arsonist's calls), `noflags` (same, never flags), `sloppy` (slow, misreads often,
misses half the pranks), `idle` (never touches the console). The bot only knows what a player could know: it learns
a call's real needs from crew reports, like you do.

## Balance results

`--headless --seeds 50`, bot `good` (E1 Golden Headset, E2 Steady Hand, E3 Night Detective, E4 Badge Returned,
E5 Burnout; "Lost/shift" = calls where help came too late):

| Difficulty | Weeks | Completed | E1 | E2 | E3 | E4 | E5 | Caught | Shifts ≥B | Avg score | Lost/shift |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Rookie | 50 | 50 | 50 | 0 | 0 | 0 | 0 | 50 | 100% | 91.9 | 0.3 |
| Dispatcher | 50 | 48 | 46 | 2 | 0 | 2 | 0 | 46 | 99% | 87.4 | 1.8 |
| Veteran | 50 | 27 | 24 | 3 | 0 | 23 | 0 | 25 | 93% | 82.0 | 3.8 |

Bot `sloppy`:

| Difficulty | Weeks | Completed | E1 | E2 | E3 | E4 | E5 | Caught | Shifts ≥B | Avg score | Lost/shift |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Rookie | 50 | 50 | 0 | 50 | 0 | 0 | 0 | 0 | 98% | 84.9 | 0.5 |
| Dispatcher | 50 | 46 | 0 | 46 | 0 | 4 | 0 | 0 | 88% | 79.7 | 2.3 |
| Veteran | 50 | 4 | 0 | 4 | 0 | 34 | 12 | 0 | 63% | 73.2 | 3.9 |

Bot `idle`: fired (E4) in the first shift on every seed and difficulty.

The `good` bot is a strong, tireless player: it never loses track of a call and always picks the nearest free unit.
Expect your own first weeks on Dispatcher to look more like the sloppy row. Veteran is hard on purpose — even the
good bot loses about half of its weeks, most often on Thursday's derby night (13 of the 23 losses above), the rest
on the Wednesday storm, the Friday finale or earlier.

## Project structure

```
dispatch-112/
├── Cargo.toml
├── README.md, LICENSE, rustfmt.toml, .gitignore
├── docs/GDD.md                 design document (story, rules, numbers)
├── src/
│   ├── main.rs                 CLI entry: TUI, --autoplay, --headless, --seeds
│   ├── lib.rs
│   ├── cli.rs                  argument parsing
│   ├── bot.rs                  scripted dispatcher (autoplay, headless, tests)
│   ├── headless.rs             week summary and balance table
│   ├── save.rs                 ~/.dispatch112/progress.json
│   ├── sim/                    pure, deterministic simulation (no terminal I/O)
│   │   ├── map.rs path.rs      city grid, districts, BFS pathfinding
│   │   ├── units.rs            vehicles and their states
│   │   ├── catalog.rs          the 69 call templates
│   │   ├── call.rs schedule.rs live calls, seeded per-night scheduler
│   │   ├── shift.rs            the tick loop: travel, escalation, resolution, meters
│   │   ├── score.rs week.rs    grades, upgrades, arsonist investigation, endings
│   │   └── config.rs rng.rs    difficulty numbers, xorshift RNG
│   ├── i18n/                   uz / ru / en: ui.json + story.json each, embedded at compile time
│   └── ui/                     ratatui front end: screens, map rendering, input loop
└── tests/
    ├── engine.rs               pathfinding, travel, wrong units, escalation, pranks, grades, endings, determinism
    ├── week_sim.rs             headless whole-week simulations
    ├── i18n.rs                 identical keys, placeholders, no empty strings
    ├── save.rs                 save/load round trip
    └── ui_smoke.rs             every screen rendered into an in-memory terminal
```

Design document: [docs/GDD.md](docs/GDD.md).

## More games by Viltrumlik

- [Karvonboshi](https://github.com/Viltrumlik/karvon-bot): Silk Road trading roguelite you play in a Telegram chat
- [Last Shore](https://github.com/Viltrumlik/last-shore): pixel-art survival city-builder on a drying sea (browser)
- [Night Express](https://github.com/Viltrumlik/night-express): text-adventure mystery on an overnight train (terminal)
- [Zero to Mars](https://github.com/Viltrumlik/zero-to-mars): 3D founder life-sim Telegram WebApp

## License

Copyright (c) 2026 Viltrumlik. All rights reserved. See [LICENSE](LICENSE).
