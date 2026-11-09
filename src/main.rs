use dispatch112::args;
use dispatch112::bot::{play_week, BotProfile};
use dispatch112::cli::{self, CliError};
use dispatch112::headless;
use dispatch112::i18n::{t, tf, Lang};
use dispatch112::sim::Difficulty;
use dispatch112::ui;

fn main() {
    let raw: Vec<String> = std::env::args().skip(1).collect();
    let opts = match cli::parse(raw.clone()) {
        Ok(o) => o,
        Err(e) => {
            // Best effort: honour --lang even when another argument is bad.
            let lang = raw
                .iter()
                .zip(raw.iter().skip(1))
                .find(|(a, _)| *a == "--lang" || *a == "-l")
                .and_then(|(_, v)| Lang::from_code(v))
                .unwrap_or_else(Lang::detect);
            let msg = match e {
                CliError::Unknown(a) => tf(lang, "cli.unknown", args!(arg = a)),
                CliError::Missing(a) => tf(lang, "cli.missing", args!(arg = a)),
                CliError::Invalid(a, v) => tf(lang, "cli.invalid", args!(arg = a, value = v)),
            };
            eprintln!("{msg}");
            eprintln!("{}", t(lang, "cli.try_help"));
            std::process::exit(2);
        }
    };
    let lang = opts.lang.unwrap_or_else(Lang::detect);
    if opts.help {
        println!("{}", t(lang, "cli.usage"));
        return;
    }
    if opts.version {
        println!("dispatch112 {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    if opts.headless || opts.seeds.is_some() {
        let bot_name = opts.bot.clone().unwrap_or_else(|| "good".to_string());
        let profile = BotProfile::from_name(&bot_name).unwrap_or_else(BotProfile::good);
        match opts.seeds {
            Some(n) => {
                let diffs: Vec<Difficulty> = match opts.difficulty {
                    Some(d) => vec![d],
                    None => Difficulty::ALL.to_vec(),
                };
                print!("{}", headless::balance_table(lang, &diffs, n, profile, &bot_name));
            }
            None => {
                let d = opts.difficulty.unwrap_or(Difficulty::Dispatcher);
                let week = play_week(d, opts.seed.unwrap_or(1), profile);
                print!("{}", headless::week_summary(lang, &week, &bot_name));
            }
        }
        return;
    }
    if let Err(e) = ui::run(&opts) {
        eprintln!("{}", tf(lang, "cli.io_error", args!(error = e)));
        std::process::exit(1);
    }
}
