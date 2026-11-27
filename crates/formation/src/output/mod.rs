//! Single owner of Formation's stdout, stderr, and diagnostic rendering.

mod common;

mod grant;

mod hook;

mod mail;

mod roster;

mod watch;

pub(crate) use common::error;
pub(crate) use common::failure;
pub(crate) use common::init_logging;
pub(crate) use grant::{
    room_list as grant_room_list, seat_list as grant_seat_list, write as grant_write,
};
pub(crate) use hook::{
    hook_error, hook_json, hook_rewake, hook_usage, prompt as hook_prompt, stop as hook_stop,
};
pub(crate) use mail::{HistoryFormat, payload as mail_payload, sent_history, write as mail_write};
pub(crate) use roster::summary;
pub(crate) use watch::{
    checked as watch_checked, exited as watch_exited, list as watch_list, payload as watch_payload,
    set_payload as watch_set_payload, write as watch_write,
};
