use crate::output::common::Mode;
use crate::output::common::mode::pick;

#[test]
fn the_flag_outranks_every_preference_and_the_terminal() {
    for setting in [None, Some("auto"), Some("human"), Some("json"), Some("x")] {
        for terminal in [false, true] {
            assert_eq!(pick(true, setting, terminal), Mode::Json);
        }
    }
}

#[test]
fn the_setting_outranks_the_terminal() {
    assert_eq!(pick(false, Some("human"), false), Mode::Human);
    assert_eq!(pick(false, Some("json"), true), Mode::Json);
    assert_eq!(pick(false, Some(" JSON "), true), Mode::Json);
}

#[test]
fn auto_follows_the_terminal() {
    assert_eq!(pick(false, Some("auto"), true), Mode::Human);
    assert_eq!(pick(false, None, false), Mode::Piped);
}

#[test]
fn unknown_values_fall_back_to_auto() {
    assert_eq!(pick(false, Some("yaml"), true), Mode::Human);
    assert_eq!(pick(false, Some(""), false), Mode::Piped);
}
