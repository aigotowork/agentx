//! Closed browser read operations; never deserialize the native aggregate input.
use serde::{Deserialize, Serialize};

use crate::daemon::observation::{InspectionRequest, InspectionResult};
use crate::observability::{configuration::*, context::*, events::*, runtime::*};

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ReadCall {
    pub expected_daemon: Option<DaemonRef>,
    pub read: BrowserRead,
}

#[derive(Deserialize, Serialize, utoipa::ToSchema)]
#[serde(deny_unknown_fields)]
pub(super) enum BrowserRead {
    Snapshot,
    Requests { select: RequestSelection, page: RuntimeList },
    RequestDetail(RequestObservationRef),
    Connections(RuntimeList),
    Peers(RuntimeList),
    Hosts(RuntimeList),
    ClientReports(RuntimeList),
    ClientReportDetail(ClientReportRef),
    ConfigurationHead { expected: Option<ObservationEpoch> },
    ConfigurationLookup(ConfigSnapshotRef),
    ConfigurationCompare(CompareConfig),
    EventsDescribe,
    EventsRead(ObservationRead),
}

#[derive(Serialize, utoipa::ToSchema)]
#[expect(clippy::large_enum_variant, reason = "One native result per capped HTTP response and at most eight in-flight reads; preserve the direct native shape and derived browser contract without lint-only indirection.")]
pub(super) enum ReadResult {
    Inspection(InspectionResult),
    LocalFailure(LocalFailure),
}

impl ReadCall {
    /// Scope is fixed at launch; references are operands, never scope selectors.
    pub fn prepare(self, scope: &ScopeRef) -> Result<(Option<DaemonRef>, InspectionRequest), &'static str> {
        if self.expected_daemon.as_ref().is_some_and(|daemon| &daemon.scope != scope) {
            return Err("ScopeMismatch");
        }
        use BrowserRead as B;
        let page_valid = |page: &RuntimeList| page.limit > 0 && page.limit <= 64 && page.before != Some(0);
        let request = match self.read {
            B::Snapshot => InspectionRequest::Runtime(RuntimeRequest::Snapshot),
            B::Requests { select, page } if page_valid(&page) => InspectionRequest::Runtime(RuntimeRequest::Requests { select, page }),
            B::RequestDetail(reference) if &reference.daemon.scope == scope && reference.id.0 > 0 => InspectionRequest::Runtime(RuntimeRequest::RequestDetail(reference)),
            B::Connections(page) if page_valid(&page) => InspectionRequest::Runtime(RuntimeRequest::Connections(page)),
            B::Peers(page) if page_valid(&page) => InspectionRequest::Runtime(RuntimeRequest::Peers(page)),
            B::Hosts(page) if page_valid(&page) => InspectionRequest::Runtime(RuntimeRequest::Hosts(page)),
            B::ClientReports(page) if page_valid(&page) => InspectionRequest::Runtime(RuntimeRequest::ClientReports(page)),
            B::ClientReportDetail(reference) if &reference.daemon.scope == scope && reference.id.0 > 0 => InspectionRequest::Runtime(RuntimeRequest::ClientReportDetail(reference)),
            B::ConfigurationHead { expected } if expected.as_ref().is_none_or(|epoch| &epoch.daemon.scope == scope) => InspectionRequest::Configuration(ConfigurationRequest::Head { expected }),
            B::ConfigurationLookup(reference) if &reference.epoch.daemon.scope == scope && reference.sequence > 0 => InspectionRequest::Configuration(ConfigurationRequest::Lookup(reference)),
            B::ConfigurationCompare(input) if &input.left.epoch.daemon.scope == scope && &input.right.epoch.daemon.scope == scope && input.left.sequence > 0 && input.right.sequence > 0 => {
                validate_selection(&input.fields).map_err(|_| "InvalidInput")?;
                InspectionRequest::Configuration(ConfigurationRequest::Compare(input))
            }
            B::EventsDescribe => InspectionRequest::Events(ObservationRequest::Describe),
            B::EventsRead(input) => {
                let filter = &input.window.filter;
                if &input.window.after.stream.epoch.daemon.scope != scope
                    || filter.configuration.as_ref().is_some_and(|reference| &reference.epoch.daemon.scope != scope)
                    || filter.domain_event.as_ref().is_some_and(|reference| &reference.scope != scope)
                {
                    return Err("ScopeMismatch");
                }
                if input.limit == 0 || input.limit > MAX_PAGE_RECORDS || filter.validate().is_err() {
                    return Err("InvalidInput");
                }
                // Native read checks era/stream before numeric frontier validity.
                return Ok((self.expected_daemon, InspectionRequest::Events(ObservationRequest::Read(input))));
            }
            _ => return Err("InvalidInput"),
        };
        Ok((self.expected_daemon, request))
    }
}
