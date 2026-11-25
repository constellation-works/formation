//! Shared execution context for offline and room commands.
mod audit;
mod exec;
pub(crate) use exec::{App, exec};
use formation_store::Store;
use std::path::PathBuf;
pub(crate) struct Ctx<'a> {
    mail: Option<(Store, PathBuf)>,
    pub(crate) actor: Option<&'a str>,
    pub(crate) clock: fn() -> time::OffsetDateTime,
}
impl Ctx<'_> {
    pub(crate) fn mail(&self) -> Result<(&Store, &std::path::Path), cw_base::Error> {
        self.mail
            .as_ref()
            .map(|(s, p)| (s, p.as_path()))
            .ok_or_else(|| {
                cw_base::Error::new(
                    cw_base::Kind::Internal,
                    "missing_context",
                    std::io::Error::other("mail context unavailable"),
                )
            })
    }
}
