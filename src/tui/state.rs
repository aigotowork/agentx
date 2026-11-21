pub use crate::service::queries::DataState;
use std::collections::BTreeSet;

use crate::tui::model::{CommandResult, Flash, InputMode, LaunchState, Overlay, RelayPopupState, ViewMode};

impl DataState {
    pub fn empty() -> Self {
        Self { agents: vec![], remote_agents: vec![], stopped_agents: vec![], orphans: vec![], messages: vec![], events: vec![], relay_enabled: false, relay_health: crate::relay::RelayHealth::NotConfigured, search_results: None }
    }
}

pub struct UiState {
    pub cursor: usize,
    pub cursor_name: Option<String>,
    pub selected: BTreeSet<String>,
    // Commands and filters use names. Inspection must retain the incarnation
    // that was actually checked, even if a later refresh reuses its name.
    pub(super) checked_subjects: std::collections::BTreeMap<String, super::info_data::Subject>,
    pub input: String,
    pub input_cursor: usize,
    /// First visible line in multi-line compose input.
    pub input_scroll: usize,
    pub flash: Option<Flash>,
    pub tick: u64,
    pub launch: LaunchState,
    pub should_quit: bool,
    pub switch_viewport: bool,
    pub msg_scroll: usize,
    /// Rendered fullscreen lines in the previous frame, for scroll anchoring.
    pub last_msg_total: usize,
    pub scroll_max: usize,
    pub search_filter: Option<String>,
    pub help_open: bool,
    pub help_scroll: u16,
    pub confirm: Option<Confirm>,
    pub mode: InputMode,
    pub command_result: Option<CommandResult>,
    pub(super) info: Option<super::info::Info>,
    pub(super) transcript: Option<super::transcript::Reader>,
    pub view_mode: ViewMode,
    pub relay_popup: Option<RelayPopupState>,
    pub relay_text_until: Option<std::time::Instant>,
    /// Whether the last observed snapshot's relay health was Connected.
    /// Drives the "relay connected" flash on the not-Connected → Connected
    /// edge. Started as false (we haven't seen any snapshot yet, so the
    /// first Connected snapshot triggers the flash, which is what we want).
    pub last_relay_was_connected: bool,
    pub remote_expanded: bool,
    pub stopped_expanded: bool,
    pub show_all_stopped: bool,
    pub orphans_expanded: bool,
    pub eject_filter: Option<BTreeSet<String>>,
    pub inline_filter_changed: bool,
    pub needs_resize: bool,
    pub needs_clear_replay: bool,
    pub overlay: Option<Overlay>,
    pub pending_eject_cmd: bool,
    /// Terminal width, updated each render frame. Used by input handlers for wrap calculations.
    pub term_width: u16,
}

impl UiState {
    /// Set flags to trigger an inline scrollback replay (filter/search change).
    /// No-op in vertical mode.
    pub fn trigger_inline_replay(&mut self) {
        if self.view_mode == ViewMode::Inline {
            self.inline_filter_changed = true;
            self.needs_resize = true;
            self.needs_clear_replay = false;
        }
    }
}

pub struct Confirm {
    pub text: String,
    pub action: ConfirmAction,
    pub selected: bool, // true = yes
    pub expires_at: std::time::Instant,
}

impl Confirm {
    pub fn new(text: String, action: ConfirmAction, default_yes: bool) -> Self {
        Self { text, action, selected: default_yes, expires_at: std::time::Instant::now() + std::time::Duration::from_secs(10) }
    }

    pub fn is_expired(&self) -> bool {
        std::time::Instant::now() >= self.expires_at
    }

    pub fn is_inline_agent_action(&self) -> bool {
        matches!(self.action, ConfirmAction::KillAgents(_) | ConfirmAction::ForkAgents(_) | ConfirmAction::ResumeAgents(_))
    }
}

pub enum ConfirmAction {
    Reset(String),
    KillAgents(Vec<String>),
    ForkAgents(Vec<String>),
    ResumeAgents(Vec<String>),
    KillOrphan(u32),
    /// Orphan chooser: selected=false → Kill, selected=true → Recover
    OrphanAction(u32),
}
