//! Originating terminal host I/O over exact participant endpoint projections.
//! The daemon returns one fenced endpoint; TCP waits and writes stay on this host.
use crate::agents::attachments::ParticipantRef;
use crate::service::{Request, Response, client::Client, queries};
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::{Duration, Instant};

/// Connect a human terminal to the exact currently registered tmux host.
pub(crate) fn attach(client: &Client, participant: &ParticipantRef) -> Result<i32, String> {
    use std::io::IsTerminal;
    if !std::io::stdin().is_terminal() || !std::io::stdout().is_terminal() {
        return Err("attach requires an interactive terminal".into());
    }
    let selected = endpoint(client, participant)?;
    let record = &selected.record;
    if record.pid.is_some_and(|pid| pid <= 0 || record.pid_reused(pid as u32) || !crate::sys::process::is_alive(pid as u32)) {
        return Err("the Agent process has stopped; use resume to start its conversation again".into());
    }
    let info = super::terminal::resolve_terminal_info(record.terminal_preset_effective.as_deref(), record.launch_context.as_deref());
    let mut command = super::tmux::attach_command(&info).map_err(|error| error.to_string())?;
    // Revalidate the participant incarnation after the external server query.
    endpoint(client, participant)?;
    command.status().map(|status| status.code().unwrap_or(1)).map_err(|error| format!("could not attach tmux: {error}"))
}

/// Wait for the new PTY host's metadata while retaining the launch reservation fence.
pub(crate) fn attach_after_launch(client: &Client, participant: &ParticipantRef) -> Result<i32, String> {
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let selected = endpoint(client, participant)?;
        let info = super::terminal::resolve_terminal_info(selected.record.terminal_preset_effective.as_deref(), selected.record.launch_context.as_deref());
        if !info.tmux.is_empty() && !info.pane_id.is_empty() {
            return attach(client, participant);
        }
        if Instant::now() >= deadline {
            return Err("timed out waiting for the tmux terminal to register".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

pub(crate) fn endpoint(client: &Client, participant: &ParticipantRef) -> Result<queries::TerminalEndpoint, String> {
    match client.call(Request::Queries(queries::Request::Terminal { participant: participant.clone() })).map_err(|error| error.to_string())? {
        Response::Queries(queries::Response::Terminal(endpoint)) => Ok(endpoint),
        _ => Err("daemon returned a different terminal endpoint".into()),
    }
}
fn require_port(endpoint: &queries::TerminalEndpoint) -> Result<i32, String> {
    endpoint.port.map(i32::from).ok_or_else(|| {
        let name = &endpoint.record.name;
        if endpoint.record.background != 0 {
            format!("'{name}' is headless, so it has no terminal screen")
        } else {
            format!("'{name}' has no terminal registered yet: it is still starting, or it runs outside an agentx-managed terminal (not launched via 'agentx launch --tool <tool>')")
        }
    })
}
pub(crate) fn inject(client: &Client, participant: &ParticipantRef, text: &str, enter: bool) -> Result<String, String> {
    let endpoint = endpoint(client, participant)?;
    let port = require_port(&endpoint)?;
    let name = &endpoint.record.name;
    if !text.is_empty() {
        inject_raw(port, text.as_bytes())?;
    }
    if enter {
        if !text.is_empty() {
            wait_for_text_rendered(port, text);
        }
        // Revalidate the admitted incarnation before the separate enter write.
        if self::endpoint(client, participant)?.port != endpoint.port {
            return Err("terminal endpoint changed before enter; text may already have been injected".into());
        }
        inject_raw(port, b"\r")?;
    }
    Ok(match (text.is_empty(), enter) {
        (false, true) => format!("Injected {} chars + enter to {}", text.len(), name),
        (false, false) => format!("Injected {} chars to {}", text.len(), name),
        (true, _) => format!("Injected enter to {}", name),
    })
}
pub(crate) fn screen(client: &Client, participant: &ParticipantRef, raw_json: bool, clean: bool) -> Result<String, String> {
    let endpoint = endpoint(client, participant)?;
    screen_endpoint(&endpoint, raw_json, clean)
}
pub(crate) fn screen_endpoint(endpoint: &queries::TerminalEndpoint, raw_json: bool, clean: bool) -> Result<String, String> {
    let port = require_port(endpoint)?;
    let name = &endpoint.record.name;
    let data = query_screen(port).ok_or_else(|| format!("No response from '{name}' (port {port})."))?;
    if raw_json { serde_json::to_string(&data).map_err(|error| error.to_string()) } else { Ok(format_screen(&data, clean)) }
}
fn inject_raw(port: i32, data: &[u8]) -> Result<(), String> {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).map_err(|e| format!("connect: {e}"))?;
    stream.set_write_timeout(Some(Duration::from_secs(2))).ok();
    stream.write_all(data).map_err(|e| format!("write: {e}"))?;
    Ok(())
}

const TEXT_RENDER_TIMEOUT: Duration = Duration::from_secs(2);

fn wait_for_text_rendered(port: i32, text: &str) {
    let deadline = Instant::now() + TEXT_RENDER_TIMEOUT;
    while Instant::now() < deadline {
        if let Some(screen) = query_screen(port)
            && screen.get("input_text").and_then(|v| v.as_str()) == Some(text)
        {
            return;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
}

fn query_screen(port: i32) -> Option<serde_json::Value> {
    let mut stream = TcpStream::connect(format!("127.0.0.1:{port}")).ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(2))).ok();
    stream.write_all(b"\x00SCREEN\n").ok()?;
    stream.shutdown(std::net::Shutdown::Write).ok()?;

    let mut data = Vec::new();
    stream.read_to_end(&mut data).ok()?;
    if data.is_empty() {
        return None;
    }
    serde_json::from_slice(&data).ok()
}

fn format_screen(data: &serde_json::Value, clean: bool) -> String {
    let lines = data["lines"].as_array();
    let cursor = data["cursor"].as_array();
    let size = data["size"].as_array();

    let (rows, cols) = size.map(|s| (s.first().and_then(|v| v.as_i64()).unwrap_or(0), s.get(1).and_then(|v| v.as_i64()).unwrap_or(0))).unwrap_or((0, 0));

    let (cr, cc) = cursor.map(|c| (c.first().and_then(|v| v.as_i64()).unwrap_or(0), c.get(1).and_then(|v| v.as_i64()).unwrap_or(0))).unwrap_or((0, 0));

    let ready = data.get("ready");
    let prompt_empty = data.get("prompt_empty");
    let input_text = data.get("input_text");

    let mut out = Vec::new();
    if !clean {
        out.push(format!("Screen {rows}x{cols}  cursor ({cr},{cc})"));
        out.push(format!(
            "ready={ready}  prompt_empty={prompt_empty}  input_text={input_text}",
            ready = ready.map(|v| v.to_string()).unwrap_or("null".into()),
            prompt_empty = prompt_empty.map(|v| v.to_string()).unwrap_or("null".into()),
            input_text = input_text
                .map(|v| match v.as_str() {
                    Some(s) => format!("\"{}\"", s),
                    None => v.to_string(),
                })
                .unwrap_or("null".into()),
        ));
        out.push(String::new());
    }

    if let Some(lines) = lines {
        for (i, line) in lines.iter().enumerate() {
            let text = line.as_str().unwrap_or("");
            if clean {
                out.push(text.to_string());
            } else if !text.is_empty() {
                out.push(format!("  {i:3}: {text}"));
            }
        }
    }

    out.join("\n")
}
