//! Shared execution context for offline and room commands.

mod audit;

mod exec;

use crate::composition::MailContext;
pub(crate) use audit::record_delivery;
pub(crate) use exec::{App, RoomAccess, exec};

pub(crate) struct Ctx<'a> {
    pub(crate) env: &'a crate::Env,
    pub(crate) mail: Option<MailContext>,
    pub(crate) actor: Option<&'a str>,
    pub(crate) cwd: Option<&'a std::path::Path>,
    pub(crate) clock: fn() -> time::OffsetDateTime,
}

impl Ctx<'_> {
    pub(crate) fn mail(&self) -> Result<&MailContext, cw_base::Error> {
        self.mail.as_ref().ok_or_else(|| {
            cw_base::Error::new(
                cw_base::Kind::Internal,
                "missing_context",
                std::io::Error::other("mail context unavailable"),
            )
        })
    }
}
