//! The published epoch-3 and epoch-4 lifecycle slice. Only explicit restart
//! enters this module; no business request is ever serialized here.
use std::path::PathBuf;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use super::{DaemonError, DaemonInfo, DaemonScope, frame, platform, transport, wire};
use crate::observability::runtime::ObservedProcess;

#[derive(Serialize, Deserialize)]
pub(super) enum LegacyCall {
    Hello { protocol_version: u32, build_version: String, build_id: String, canonical_scope: PathBuf },
    Shutdown { daemon_instance: String },
}

#[derive(Serialize, Deserialize)]
pub(super) enum LegacyReply {
    Ready(DaemonInfo),
    Stopping,
    Failure(DaemonError),
}

/// Retain the Ready socket. Verify the authenticated peer and exact scope
/// before using its instance token; then ask the same connection to drain.
pub(super) fn shutdown(scope: &DaemonScope, peer: &(u32, Option<String>), deadline: Instant) -> Result<(), DaemonError> {
    for epoch in [4, 3] {
        let probe_end = deadline.min(Instant::now() + transport::STATUS_TIMEOUT);
        let mut stream = platform::connect_existing(scope, probe_end)?;
        if &platform::peer(&stream)? != peer {
            return Err(DaemonError::Incompatible("legacy daemon peer changed during restart".into()));
        }
        let hello = LegacyCall::Hello { protocol_version: epoch, build_version: wire::BUILD_VERSION.into(), build_id: wire::build_identity().into(), canonical_scope: scope.root().to_path_buf() };
        frame::write(&mut stream, &hello, probe_end)?;
        let info = match frame::read(&mut stream, probe_end)? {
            LegacyReply::Ready(info) => info,
            LegacyReply::Failure(DaemonError::Incompatible(_)) if epoch == 4 => continue,
            LegacyReply::Failure(error) => return Err(error),
            _ => return Err(DaemonError::Protocol("expected legacy daemon readiness".into())),
        };
        transport::validate_info(scope, &ObservedProcess { pid: peer.0, incarnation: peer.1.clone() }, &info)?;
        if info.protocol_version != epoch || &platform::peer(&stream)? != peer {
            return Err(DaemonError::Incompatible("legacy epoch or peer changed during restart".into()));
        }
        let unknown = |error: DaemonError| DaemonError::OutcomeUnknown(format!("legacy shutdown of {}: {error}", info.daemon_instance));
        let call = LegacyCall::Shutdown { daemon_instance: info.daemon_instance.clone() };
        frame::write(&mut stream, &call, deadline).map_err(unknown)?;
        return match frame::read(&mut stream, deadline).map_err(unknown)? {
            LegacyReply::Stopping => Ok(()),
            LegacyReply::Failure(error) => Err(error),
            _ => Err(unknown(DaemonError::Protocol("expected legacy shutdown acknowledgement".into()))),
        };
    }
    unreachable!("both published epochs handled")
}
