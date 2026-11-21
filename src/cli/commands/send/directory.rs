//! CLI selection/presentation adapter for shared native directory orchestration.
use crate::cli::client::Session;
use crate::config::roles::DefaultRoute;
pub(super) use crate::runtime::directory::Address;
use anyhow::Result;

pub(super) fn resolve(session: &Session, dir: Option<&str>, workspace: Option<&str>) -> Result<(Address, DefaultRoute)> {
    let selected = crate::cli::workspace::selected(session, dir, workspace)?;
    let requested = match dir {
        Some(dir) => session.invocation.cwd.join(dir).canonicalize()?,
        None => selected.clone().unwrap_or_else(|| session.invocation.cwd.clone()),
    };
    crate::runtime::directory::resolve(&session.client, &requested, selected.as_deref())
}

pub(super) fn recipient(session: &Session, address: &mut Address) -> Result<String> {
    let launcher = crate::cli::commands::launch::launcher_name_client(session);
    crate::runtime::directory::recipient(&session.client, address, &launcher).map(|participant| participant.canonical_name)
}
