//! The one output-mode resolver for commands that offer a human and a machine view.

use std::io::{self, IsTerminal};

/// Environment variable that sets the output preference: `auto`, `human` or `json`.
const FORMAT_VAR: &str = "FORMATION_FORMAT";

/// How a command renders its result.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Mode {
    /// One JSON document, chosen by `--json` or `FORMATION_FORMAT=json`.
    Json,
    /// Readable output for a person, chosen by `FORMATION_FORMAT=human` or a terminal.
    Human,
    /// The established form when stdout is not a terminal and nothing was asked for.
    Piped,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Preference {
    Auto,
    Human,
    Json,
}

impl Preference {
    /// Unset and unrecognized values mean `auto`.
    fn parse(value: Option<&str>) -> Self {
        match value
            .map(|text| text.trim().to_ascii_lowercase())
            .as_deref()
        {
            Some("human") => Self::Human,
            Some("json") => Self::Json,
            _ => Self::Auto,
        }
    }
}

/// Resolve the mode once per invocation: `--json`, then `FORMATION_FORMAT`, then the terminal.
pub(crate) fn resolve(json_flag: bool) -> Mode {
    let setting = std::env::var(FORMAT_VAR).ok();
    pick(json_flag, setting.as_deref(), io::stdout().is_terminal())
}

pub(super) fn pick(json_flag: bool, setting: Option<&str>, terminal: bool) -> Mode {
    if json_flag {
        return Mode::Json;
    }
    match Preference::parse(setting) {
        Preference::Json => Mode::Json,
        Preference::Human => Mode::Human,
        Preference::Auto if terminal => Mode::Human,
        Preference::Auto => Mode::Piped,
    }
}
