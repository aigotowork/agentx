//! Shared types, constants, and utilities for agentx.

pub mod ansi;
pub mod constants;
use crate::runtime::context;
pub mod errors;
pub mod identity;

pub mod platform;
pub mod suggest;
pub mod time;

// Re-export key types at module level for convenience.
pub use crate::integrations::tool::Tool;
pub use crate::runtime::terminal::presets::{TerminalPreset, get_terminal_preset};
pub use constants::{
    MAX_MESSAGE_SIZE,
    MAX_MESSAGES_PER_DELIVERY,
    // Patterns
    MENTION_PATTERN,
    // Message constants
    SENDER,
    // Status constants
    ST_ACTIVE,
    ST_BLOCKED,
    ST_ERROR,
    ST_INACTIVE,
    ST_LAUNCHING,
    ST_LISTENING,
    SYSTEM_SENDER,
    TitleMode,
    VALID_TITLE_MODES,
    // Functions
    extract_mentions,
    format_pane_title,
    format_pane_title_combined,
    status_bg,
    status_fg,
    status_icon,
};
pub use context::AgentxContext;
pub use errors::{AgentxError, CLIError, HookError};
pub use identity::{CommandContext, SenderIdentity, SenderKind};
pub use platform::{detect_current_tool_from_env, dev_root_binary, is_inside_ai_tool, is_termux, is_wsl, platform_name, shorten_path, shorten_path_max};
pub use time::{format_age, now_epoch_f64, now_epoch_i64, system_time_to_epoch_f64};

pub(crate) mod serde_schema;
