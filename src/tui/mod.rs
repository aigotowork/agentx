pub mod actions;
mod agent_mouse;
pub mod app;
pub mod commands;
mod control_mouse;
pub mod data;
pub mod db;
mod info;
mod info_data;
pub mod inline;
pub mod input;
mod launch;
pub mod model;
mod mouse;
pub mod render;
#[cfg(test)]
mod replay;
pub mod rpc;
pub(crate) mod select;

pub mod state;
pub mod theme;
mod transcript;
mod usage;

use std::io::{Write, stdout};

use anyhow::Result;
use crossterm::event::{DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture};
use crossterm::execute;
use crossterm::terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode};
use ratatui::backend::CrosstermBackend;
use ratatui::{Terminal, TerminalOptions, Viewport};

use self::app::App;
use self::inline::eject::ReplayReason;
use self::model::ViewMode;

const DEFAULT_INLINE_HEIGHT: u16 = 13;

/// Clear the inline viewport area and reset scroll region.
/// Uses current terminal size — only valid when terminal hasn't been resized
/// since the viewport was created (viewport switches, quit).
fn clear_viewport_area(viewport_height: u16) -> Result<()> {
    let mut out = stdout();
    let (_, rows) = crossterm::terminal::size()?;
    let viewport_start = rows.saturating_sub(viewport_height);
    write!(out, "\x1b[r")?;
    write!(out, "\x1b[{};1H", viewport_start + 1)?;
    write!(out, "\x1b[J")?;
    out.flush()?;
    Ok(())
}

/// Clean up after a resize: clear the entire visible screen so replay doesn't
/// duplicate previously ejected content. Content already in the terminal's
/// scrollback buffer (scrolled off-screen) is unreachable without \x1b[3J and
/// left as-is to preserve pre-TUI shell history.
fn clear_for_resize(viewport_height: u16) -> Result<()> {
    let mut out = stdout();
    let (_, new_rows) = crossterm::terminal::size()?;
    let new_start = new_rows.saturating_sub(viewport_height);

    write!(out, "\x1b[r")?; // reset scroll region
    // Clear only the area above the inline viewport. Avoid a full-screen visible
    // wipe, which causes an obvious flash before replay redraws.
    for row in 1..=new_start {
        write!(out, "\x1b[{};1H\x1b[2K", row)?;
    }
    write!(out, "\x1b[{};1H", new_start + 1)?; // park at new viewport start
    out.flush()?;
    Ok(())
}

fn prepare_vertical_viewport(app: &mut App) {
    app.ui.switch_viewport = false;
    app.ui.view_mode = ViewMode::Vertical;
    app.source.set_timeline_limit(5000);
    app.ui.eject_filter = None;
    app.ui.msg_scroll = 0;
}

fn prepare_inline_viewport(app: &mut App) {
    app.ui.switch_viewport = false;
    app.ui.view_mode = ViewMode::Inline;
    app.source.set_timeline_limit(200);
    app.ui.eject_filter = None;
    app.ui.msg_scroll = 0;
}

fn run_fullscreen(app: &mut App, viewport_height: u16, in_alt_screen: &mut bool) -> Result<()> {
    execute!(stdout(), EnterAlternateScreen)?;
    *in_alt_screen = true;
    execute!(stdout(), EnableMouseCapture, EnableBracketedPaste)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;
    app.run(&mut terminal, viewport_height, false)?;
    drop(terminal);
    execute!(stdout(), LeaveAlternateScreen)?;
    *in_alt_screen = false;
    // Reassert input modes when the main terminal buffer is restored.
    execute!(stdout(), EnableMouseCapture, EnableBracketedPaste)?;
    Ok(())
}

/// Main event loop. Separated from run() so cleanup always runs regardless
/// of errors. `in_alt_screen` tracks whether we entered the alternate screen
/// so the caller can tear it down.
fn run_app(app: &mut App, viewport_height: u16, in_alt_screen: &mut bool) -> Result<()> {
    loop {
        match app.ui.view_mode {
            ViewMode::Inline => {
                *in_alt_screen = false;
                let backend = CrosstermBackend::new(stdout());
                let mut terminal = match Terminal::with_options(backend, TerminalOptions { viewport: Viewport::Inline(viewport_height) }) {
                    Ok(t) => t,
                    Err(e) => {
                        return Err(anyhow::anyhow!(
                            "failed to initialize inline viewport: {}\n\
                             Try running in a terminal with full ANSI support.",
                            e
                        ));
                    }
                };

                loop {
                    app.run(&mut terminal, viewport_height, true)?;
                    if !app.reader_open() || app.ui.should_quit || app.ui.needs_resize || app.ui.switch_viewport {
                        break;
                    }
                    // Keep the event scrollback intact while the stacked layout renders the reader above the Agent strip.
                    let size = crossterm::terminal::size()?;
                    run_fullscreen(app, viewport_height, in_alt_screen)?;
                    if crossterm::terminal::size()? != size {
                        app.ui.needs_resize = true;
                    }
                }
                // Ratatui anchors the inline viewport at the cursor row, not the
                // screen bottom, so take its real origin before dropping it.
                let viewport_top = terminal.get_frame().area().y;
                drop(terminal);

                if app.ui.should_quit {
                    // Erase the box so the shell prompt starts below the timeline
                    // instead of drawing over the last frame.
                    let mut out = stdout();
                    write!(out, "\x1b[r\x1b[{};1H\x1b[J", viewport_top + 1)?;
                    out.flush()?;
                    break;
                }

                if app.ui.needs_resize {
                    app.ui.needs_resize = false;
                    app.ui.needs_clear_replay = true;
                    app.ejector.begin_replay(&app.data, &app.ui.eject_filter, &app.ui.search_filter, ReplayReason::Resize);
                    continue;
                }

                // Viewport switch (no resize) — terminal size unchanged.
                clear_viewport_area(viewport_height)?;
                prepare_vertical_viewport(app);
            }

            ViewMode::Vertical => {
                run_fullscreen(app, viewport_height, in_alt_screen)?;

                if app.ui.should_quit {
                    break;
                }

                // Clear the viewport area so the new inline terminal starts clean.
                clear_viewport_area(viewport_height)?;
                prepare_inline_viewport(app);
            }
        }
    }
    Ok(())
}

/// Inner TUI lifecycle — everything after raw mode is enabled.
fn run_inner(viewport_height: u16, filter: crate::service::queries::AgentFilter) -> Result<()> {
    execute!(stdout(), EnableMouseCapture, EnableBracketedPaste)?;

    // Save current title (push stack) and set agentx title
    {
        let mut out = stdout();
        let _ = write!(out, "\x1b[22;0t\x1b]0;agentx\x07");
        let _ = out.flush();
    }

    let mut app = App::with_filter(filter);

    // Auto-spawn relay-worker if relay is configured
    crate::relay::worker::try_spawn_worker();

    // AGENTX_TUI_FULLSCREEN=1 starts directly in alternate screen (fullscreen) mode,
    // bypassing inline viewport which requires cursor position queries.
    if std::env::var("AGENTX_TUI_FULLSCREEN").as_deref() == Ok("1") {
        app.ui.view_mode = self::model::ViewMode::Vertical;
        app.source.set_timeline_limit(5000);
    }

    let mut in_alt_screen = false;

    let result = run_app(&mut app, viewport_height, &mut in_alt_screen);

    if in_alt_screen {
        let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen);
    }
    let _ = execute!(stdout(), DisableBracketedPaste);

    // Restore terminal title (pop stack, or reset to empty)
    {
        let mut out = stdout();
        let _ = write!(out, "\x1b[23;0t"); // xterm pop title
        let _ = out.flush();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::data::DataSource;
    use crate::tui::model::Agent;
    use crate::tui::state::DataState;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::cell::Cell;
    use std::rc::Rc;

    struct DummySource {
        snapshot: DataState,
        timeline_limit: Rc<Cell<usize>>,
    }

    impl DummySource {
        fn new(snapshot: DataState, timeline_limit: Rc<Cell<usize>>) -> Self {
            Self { snapshot, timeline_limit }
        }
    }

    impl DataSource for DummySource {
        fn load(&mut self) -> DataState {
            self.snapshot.clone()
        }

        fn load_if_changed(&mut self) -> Option<DataState> {
            Some(self.snapshot.clone())
        }

        fn load_all_stopped(&mut self) -> Vec<Agent> {
            self.snapshot.stopped_agents.clone()
        }

        fn set_timeline_limit(&mut self, limit: usize) {
            self.timeline_limit.set(limit);
        }
    }

    fn make_test_app(limit: Rc<Cell<usize>>) -> App {
        crate::config::Config::init();
        let mut app = crate::tui::test_helpers::offline_app();
        let data = DataState::empty();
        app.data = data.clone();
        app.source = Box::new(DummySource::new(data, limit));
        app
    }

    fn render_once(app: &mut App) {
        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|frame| render::render(frame, app, DEFAULT_INLINE_HEIGHT)).unwrap();
    }

    #[test]
    fn model_request_error_renders_yellow_in_actual_agent_row() {
        let mut app = crate::tui::test_helpers::offline_app();
        let mut agent = crate::tui::test_helpers::make_test_agent("kera", 0.0);
        agent.status = crate::tui::model::AgentStatus::Error;
        agent.status_context = "model:request".into();
        agent.status_detail = "upstream 503".into();
        app.data.agents.push(agent);
        let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
        terminal.draw(|frame| render::render(frame, &mut app, DEFAULT_INLINE_HEIGHT)).unwrap();
        let buffer = terminal.backend().buffer();
        assert!(buffer.content.iter().any(|cell| cell.symbol() == "!" && cell.fg == crate::tui::theme::palette::YELLOW));
        let text: String = buffer.content.iter().map(|cell| cell.symbol()).collect();
        assert!(text.contains("kera"));
    }

    #[test]
    fn inline_transition_sets_vertical_mode_and_limit() {
        let limit = Rc::new(Cell::new(0));
        let mut app = make_test_app(limit.clone());
        app.ui.view_mode = ViewMode::Inline;
        app.ui.switch_viewport = true;

        render_once(&mut app);
        prepare_vertical_viewport(&mut app);

        assert_eq!(app.ui.view_mode, ViewMode::Vertical);
        assert_eq!(app.ui.msg_scroll, 0);
        assert!(!app.ui.switch_viewport);
        assert_eq!(limit.get(), 5000);
    }

    #[test]
    fn vertical_transition_resets_inline_state() {
        let limit = Rc::new(Cell::new(0));
        let mut app = make_test_app(limit.clone());
        app.ui.view_mode = ViewMode::Vertical;
        app.ui.msg_scroll = 7;
        app.ui.switch_viewport = true;

        render_once(&mut app);
        prepare_inline_viewport(&mut app);

        assert_eq!(app.ui.view_mode, ViewMode::Inline);
        assert_eq!(app.ui.msg_scroll, 0);
        assert!(!app.ui.switch_viewport);
        assert_eq!(limit.get(), 200);
    }
}

#[cfg(test)]
pub mod test_helpers {
    use crate::integrations::tool::Tool;
    use crate::tui::app::App;
    use crate::tui::data::DataSource;
    use crate::tui::model::{Agent, AgentStatus, epoch_now};
    use crate::tui::state::DataState;

    struct NullSource;
    impl DataSource for NullSource {
        fn load(&mut self) -> DataState {
            DataState::empty()
        }
        fn load_all_stopped(&mut self) -> Vec<Agent> {
            vec![]
        }
    }

    /// An `App` with no database and no RPC worker. `App::new()` would read
    /// the live `$AGENTX_DIR` db, which serial tests repoint at their own.
    pub fn offline_app() -> App {
        App::with_source_and_launch(Box::new(NullSource), None, super::model::LaunchState::with_defaults(Default::default(), vec!["default".into()]))
    }

    /// Create a test Agent with sensible defaults. `age_secs` controls how old the agent appears.
    pub fn make_test_agent(name: &str, age_secs: f64) -> Agent {
        let now = epoch_now();
        Agent {
            name: name.into(),
            tool: Tool::Claude.into(),
            status: AgentStatus::Active,
            status_context: "working".into(),
            status_detail: String::new(),
            created_at: now - age_secs,
            status_time: now,
            last_heartbeat: now,
            has_tcp: true,
            directory: "/tmp".into(),
            unread: 0,
            last_event_id: None,
            device_name: None,
            sync_age: None,
            headless: false,
            session_id: None,
            transcript_path: None,
            pid: None,
            info_reference: None,
            terminal_preset: None,
        }
    }
}

/// Restore terminal state — called from both normal cleanup and the panic hook.
fn restore_terminal() {
    let _ = disable_raw_mode();
    let _ = execute!(stdout(), DisableMouseCapture, LeaveAlternateScreen);
    let _ = execute!(stdout(), DisableBracketedPaste);
    // Restore title (pop stack)
    let mut out = stdout();
    let _ = write!(out, "\x1b[23;0t");
    let _ = out.flush();
}

/// Entry point for the default CLI dashboard.
pub(crate) fn run(filter: crate::service::queries::AgentFilter) -> Result<()> {
    // Panic hook is installed below (restore_terminal + original_hook).

    // Install panic hook that restores terminal state before printing panic info.
    // Without this, a panic leaves the terminal in raw mode and unusable.
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore_terminal();
        original_hook(info);
    }));

    let viewport_height = std::env::var("AGENTX_INLINE_HEIGHT").ok().and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_INLINE_HEIGHT);

    enable_raw_mode()?;
    let result = run_inner(viewport_height, filter);
    let _ = execute!(stdout(), DisableMouseCapture);
    let _ = disable_raw_mode();
    println!();

    result
}
