//! `agentx relay` command — cross-device sync via MQTT pub/sub.

use crate::relay::broker::BrokerProbe;
use crate::relay::token::{DecodedToken, decode_join_token, encode_join_token};
use crate::relay::{self, DEFAULT_BROKERS};
use crate::service::client::Client;
use crate::shared::ansi::{FG_GRAY, FG_GREEN, FG_RED, FG_YELLOW, RESET};
use crate::shared::time::format_age;

/// Parsed arguments for `agentx relay`.
#[derive(clap::Parser, Debug)]
#[command(name = "relay", about = "Cross-device sync via MQTT")]
pub struct RelayArgs {
    /// Subcommand and arguments (new/connect/off/status + flags)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub args: Vec<String>,
}

/// Parse --broker and --password flags from argv.
fn parse_broker_flags(argv: &[String]) -> (Option<String>, Option<String>, Vec<String>) {
    let mut broker = None;
    let mut auth_token = None;
    let mut remaining = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        if argv[i] == "--broker" && i + 1 < argv.len() {
            broker = Some(argv[i + 1].clone());
            i += 2;
        } else if argv[i] == "--password" && i + 1 < argv.len() {
            auth_token = Some(argv[i + 1].clone());
            i += 2;
        } else {
            remaining.push(argv[i].clone());
            i += 1;
        }
    }
    (broker, auth_token, remaining)
}

/// Ping a broker via TCP connect (+ TLS handshake when use_tls=true).
/// Returns round-trip ms or None on failure.
fn ping_broker(host: &str, port: u16, use_tls: bool) -> Option<u32> {
    crate::relay::broker::ping_broker(host, port, use_tls).map(|ms| ms as u32)
}

/// Build the "Add devices" / "current group" join token from current config,
/// or `None` if the relay isn't fully configured.
fn current_join_token(config: &crate::config::AgentxConfig) -> Option<String> {
    if config.relay_id.is_empty() || config.relay.is_empty() {
        return None;
    }
    let psk = relay::load_psk(config).ok()?;
    encode_join_token(&config.relay_id, &config.relay, &psk)
}

/// Show the join token (separate command to avoid leaking in status output).
fn relay_show_token() -> i32 {
    let invocation = match crate::service::InvocationContext::capture() {
        Ok(invocation) => invocation,
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    let (scope, _) = crate::paths::resolve_agentx_dir_from_env(&invocation.environment, &invocation.cwd);
    let config = crate::config::AgentxConfig::load_for_scope(&scope, &invocation.environment);
    match current_join_token(&config) {
        Some(token) => {
            println!("agentx relay connect {token}");
            if !config.relay_token.is_empty() {
                println!("  (also needs: --password <secret>)");
            }
            0
        }
        None => {
            eprintln!("No relay configured. Run: agentx relay new");
            1
        }
    }
}

fn validate_existing_relay_config(config: &crate::config::AgentxConfig) -> Result<(), String> {
    if config.relay_id.is_empty() {
        return Err("no relay configured".to_string());
    }
    if config.relay.trim().is_empty() {
        return Err("relay broker URL is missing".to_string());
    }
    if relay::parse_broker_url(&config.relay).is_none() {
        return Err(format!("relay broker URL is invalid: {}", config.relay));
    }
    relay::load_psk(config).map(|_| ())
}

/// Format a timestamp as relative age.
fn format_time(timestamp: f64) -> String {
    if timestamp == 0.0 {
        return "never".to_string();
    }
    let now = crate::shared::time::now_epoch_f64();
    let age = (now - timestamp) as i64;
    if age <= 0 {
        return "just now".to_string();
    }
    format!("{} ago", format_age(age))
}

/// Get device short ID via FNV-1a hash
/// Auto-creates device_id file if missing (via read_device_uuid).
/// Returns "?" when the device_id file cannot be created or read — display only.
fn get_device_short_id(client: &Client) -> Result<String, String> {
    let Some(device_uuid) = relay::read_device_uuid_in(client.scope_root()) else {
        return Ok("?".into());
    };
    match relay::persistence::call(client, relay::service::Request::ShortId { device_uuid }).map_err(|error| error.to_string())? {
        relay::service::Response::ShortId(short) => Ok(short),
        _ => Err("daemon returned a different device short ID response".into()),
    }
}

fn config_for(client: &Client) -> crate::config::AgentxConfig {
    crate::config::AgentxConfig::load_for_scope(client.scope_root(), &client.invocation().environment)
}

fn paint(code: &str, text: &str) -> String {
    use std::io::IsTerminal;

    let enabled = std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none() && std::env::var("TERM").as_deref() != Ok("dumb");
    if enabled { format!("{code}{text}{RESET}") } else { text.to_string() }
}

fn is_public_broker(broker: &str) -> bool {
    DEFAULT_BROKERS.iter().any(|&(h, p)| broker == format!("mqtts://{h}:{p}") || broker == format!("mqtt://{h}:{p}"))
}

/// Hint for a broker error that looks like rejected credentials on a private broker.
fn auth_hint(config: &crate::config::AgentxConfig, error: Option<&str>) -> Option<&'static str> {
    let err = error.unwrap_or("").to_ascii_lowercase();
    let auth_error = err.contains("password") || err.contains("auth") || err.contains("not authorized");
    if !auth_error || is_public_broker(&config.relay) {
        return None;
    }
    Some(if config.relay_token.is_empty() { "this broker needs a password: agentx relay connect <token> --password <secret>" } else { "broker rejected the password: agentx relay connect <token> --password <secret>" })
}

/// Peer agents shown per device in `agentx relay` before collapsing to "+N".
const MAX_LISTED_AGENTS: usize = 5;

/// Show relay status.
fn relay_status(client: &Client) -> i32 {
    let config = config_for(client);

    if config.relay_id.is_empty() {
        println!("{}", paint(FG_GRAY, "Relay: not configured"));
        println!("Run: agentx relay new");
        return 0;
    }

    if !config.relay_enabled {
        println!("{}", paint(FG_YELLOW, "Relay: disabled"));
        println!("\nRun: agentx relay on");
        return 0;
    }

    // All status display branches off the canonical RelayHealth derivation so
    // CLI / TUI / JSON can't drift in interpretation of the underlying KV.
    let health = match relay::relay_health_client(&config, client) {
        Ok(health) => health,
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    match &health {
        relay::RelayHealth::Connected => {
            println!("Status:    {}", paint(FG_GREEN, "connected"));
        }
        relay::RelayHealth::Starting { pid } => {
            println!("Status:    {} (PID {pid}, awaiting connect)", paint(FG_YELLOW, "starting"));
        }
        relay::RelayHealth::Stale { age_s, pid } => {
            println!("Status:    {} — worker unresponsive (PID {pid}, {:.0}s since last heartbeat)", paint(FG_YELLOW, "stale"), age_s);
            println!("  Fix: agentx relay daemon restart");
        }
        relay::RelayHealth::Waiting => {
            println!("Status:    {} — daemon not running (start: agentx relay daemon start)", paint(FG_YELLOW, "stopped"));
        }
        relay::RelayHealth::Error { reason, detail, pid } => {
            println!("Status:    {} — {}", paint(FG_RED, "error"), reason.clone().label(detail.as_deref(), *pid));
            match reason {
                relay::RelayErrorReason::Reported => match auth_hint(&config, detail.as_deref()) {
                    Some(hint) => println!("  Hint: {hint}"),
                    None => println!("  If this persists: agentx relay daemon restart"),
                },
                relay::RelayErrorReason::StalePidfile | relay::RelayErrorReason::Ghost => {
                    println!("  Fix: agentx relay daemon restart");
                }
            }
        }
        // NotConfigured / Disabled never reach here — the early returns above
        // (config.relay_id empty / !config.relay_enabled) handled them already.
        relay::RelayHealth::NotConfigured | relay::RelayHealth::Disabled => {
            println!("Status:    {}", paint(FG_YELLOW, "waiting"));
        }
    }

    // Broker info
    if !config.relay.is_empty() {
        if let Some((host, port, use_tls)) = relay::parse_broker_url(&config.relay) {
            if let Some(ms) = ping_broker(&host, port, use_tls) {
                println!("Broker:    {} ({ms}ms)", config.relay);
            } else {
                println!("Broker:    {} (unreachable)", config.relay);
            }
        } else {
            println!("Broker:    {}", config.relay);
        }
    } else {
        println!("Broker:    auto (public fallback)");
    }

    let own_device = relay::read_device_uuid_in(client.scope_root()).unwrap_or_default();
    let snapshot = match relay::snapshot_client(client, &own_device) {
        Ok(snapshot) => snapshot,
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    if snapshot.queued > 0 {
        println!("Queued:    {} events pending", snapshot.queued);
    } else {
        println!("Queued:    up to date");
    }
    let state = match relay::get_relay_status_client(&config, client) {
        Ok(state) => state,
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    println!("Last push: {}", format_time(state.last_push));

    let now = crate::shared::time::now_epoch_f64();
    // Keep one display row per device while retaining every short alias in
    // the native snapshot for `off --all` notification.
    let mut devices = std::collections::HashMap::new();
    for peer in snapshot.peers {
        devices.insert(peer.device_uuid.clone(), peer);
    }
    let device_short = match get_device_short_id(client) {
        Ok(short) => short,
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    println!("\nDevice:    {device_short}");
    let mut sorted_devices: Vec<_> = devices.into_values().collect();
    sorted_devices.sort_by(|a, b| a.short.cmp(&b.short));
    sorted_devices.sort_by_key(|peer| !(peer.sync_time > 0.0 && now - peer.sync_time <= relay::DEVICE_STALE_SECS));
    if sorted_devices.is_empty() {
        println!("Peers:     none yet — join from another device (token: agentx relay token)");
    } else {
        println!("Peers:");
        for peer in sorted_devices {
            let is_online = peer.sync_time > 0.0 && now - peer.sync_time <= relay::DEVICE_STALE_SECS;
            let state = if is_online {
                paint(FG_GREEN, "online")
            } else if peer.sync_time > 0.0 {
                paint(FG_GRAY, &format!("seen {}", format_time(peer.sync_time)))
            } else {
                paint(FG_GRAY, "not seen")
            };
            let agents = if peer.agents.len() > MAX_LISTED_AGENTS {
                format!("{} +{}", peer.agents[..MAX_LISTED_AGENTS].join(", "), peer.agents.len() - MAX_LISTED_AGENTS)
            } else if !peer.agents.is_empty() {
                peer.agents.join(", ")
            } else if is_online {
                "no agents".into()
            } else {
                String::new()
            };
            let line = format!("  {}  {state}  {agents}", peer.short);
            println!("{}", line.trim_end());
        }
    }

    // Show key fingerprint so two devices can visually verify they share the
    // same PSK without anyone leaking material.
    if let Ok(psk) = relay::load_psk(&config) {
        println!("Key:       {}", relay::crypto::fingerprint(&psk));
    } else {
        println!("Key:       {} (run `agentx relay new`)", paint(FG_RED, "missing"));
    }

    println!("\nShow token: agentx relay token");

    0
}

/// Internal fast-path: wake the relay worker so queued events publish immediately.
fn relay_push(client: &Client) -> i32 {
    crate::relay::trigger_push_client(client);
    0
}

fn restart_relay_worker_for_config_change() -> bool {
    if crate::relay::worker::is_relay_worker_running() {
        crate::relay::worker::stop_relay_worker_blocking();
    }
    crate::relay::worker::ensure_worker()
}

/// How long `relay new/connect/on` wait for the broker handshake before
/// reporting back. Covers TLS + MQTT connect on a slow public broker.
const CONNECT_WAIT: std::time::Duration = std::time::Duration::from_secs(8);

/// Poll until the worker reports connected or a broker error, or the wait runs out.
fn await_broker_connection(client: &Client) -> Result<relay::RelayHealth, String> {
    let config = config_for(client);
    let deadline = std::time::Instant::now() + CONNECT_WAIT;
    loop {
        let health = relay::relay_health_client(&config, client).map_err(|error| error.to_string())?;
        let settled = matches!(health, relay::RelayHealth::Connected | relay::RelayHealth::Error { .. });
        if settled || std::time::Instant::now() >= deadline {
            return Ok(health);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Wait for the broker, then show full status. Fails on a broker error so
/// `relay on` exits like `relay new/connect` do.
fn status_after_connect(client: &Client) -> i32 {
    let health = match await_broker_connection(client) {
        Ok(health) => health,
        Err(error) => {
            eprintln!("Error: {error}");
            return 1;
        }
    };
    let rc = relay_status(client);
    if matches!(health, relay::RelayHealth::Error { .. }) {
        return 1;
    }
    rc
}

/// Restart the worker on new relay config and report whether it reached the
/// broker — not just whether a process started.
fn start_worker_and_report(client: &Client) -> i32 {
    if !restart_relay_worker_for_config_change() && !crate::relay::worker::is_relay_worker_running() {
        eprintln!("\nCould not start daemon automatically. Run 'agentx relay daemon start'.");
        return 1;
    }
    match await_broker_connection(client) {
        Err(error) => {
            eprintln!("Error: {error}");
            1
        }
        Ok(relay::RelayHealth::Connected) => {
            println!("\n{}", paint(FG_GREEN, "Connected."));
            0
        }
        Ok(relay::RelayHealth::Error { reason, detail, pid }) => {
            let config = config_for(client);
            eprintln!("\nNot connected: {}", reason.label(detail.as_deref(), pid));
            if let Some(hint) = auth_hint(&config, detail.as_deref()) {
                eprintln!("  Hint: {hint}");
            }
            eprintln!("The daemon keeps retrying. Check with: agentx relay");
            1
        }
        _ => {
            println!("\nDaemon running, broker not reached yet ({}s). Check with: agentx relay", CONNECT_WAIT.as_secs());
            0
        }
    }
}

fn ensure_relay_worker_running_for_cli() -> bool {
    if crate::relay::worker::ensure_worker() {
        return true;
    }

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
    while std::time::Instant::now() < deadline {
        if crate::relay::worker::is_relay_worker_running() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
        if crate::relay::worker::ensure_worker() {
            return true;
        }
    }

    crate::relay::worker::is_relay_worker_running()
}

fn known_remote_device_shorts(client: &Client) -> Result<Vec<String>, String> {
    let own = relay::read_device_uuid_in(client.scope_root()).unwrap_or_default();
    let snapshot = relay::snapshot_client(client, &own).map_err(|error| error.to_string())?;
    let mut shorts: Vec<_> = snapshot.peers.into_iter().map(|peer| peer.short).collect();
    shorts.sort();
    Ok(shorts)
}

fn relay_notify_off_all(client: &Client, config: &crate::config::AgentxConfig) -> Result<(), String> {
    let peers = known_remote_device_shorts(client)?;
    if peers.is_empty() {
        println!("No known remote peers to notify.");
        return Ok(());
    }
    let sent = peers.iter().filter(|short| relay::control::send_one_way_control_ephemeral(client, config, "relay_off", short, &serde_json::json!({}))).count();
    println!("Best-effort relay_off sent to {sent}/{} known remote peer(s).", peers.len());
    Ok(())
}

fn relay_off(client: &Client, argv: &[String]) -> i32 {
    let all = argv.iter().any(|a| a == "--all");
    if let Some(extra) = argv.iter().find(|a| *a != "--all") {
        return crate::cli::commands::help::usage_error("relay", &format!("Unexpected argument for agentx relay off: '{extra}'"));
    }

    let config = config_for(client);
    if config.relay_id.is_empty() {
        eprintln!("No relay configured.");
        eprintln!("Run: agentx relay new");
        return 1;
    }

    if all {
        if config.relay_enabled {
            if let Err(error) = relay_notify_off_all(client, &config) {
                eprintln!("Error: {error}");
                return 1;
            }
        } else {
            println!("Relay already disabled locally; skipping remote shutdown broadcast.");
        }
    }

    match relay::control::disable_local_relay(&config, client) {
        Ok(cleared_remote_state) => {
            if cleared_remote_state {
                println!("Cleared remote state");
            }
        }
        Err(e) => {
            eprintln!("Error: {e}");
            return 1;
        }
    }

    crate::relay::worker::stop_relay_worker_blocking();
    println!("{}", paint(FG_YELLOW, "Relay: disabled"));
    println!("\nRun 'agentx relay on' to reconnect");
    0
}

/// Enable or disable relay sync.
fn relay_toggle(client: &Client, enable: bool) -> i32 {
    let config = config_for(client);

    if config.relay_id.is_empty() {
        eprintln!("No relay configured.");
        eprintln!("Run: agentx relay new");
        return 1;
    }

    if enable {
        if let Err(e) = validate_existing_relay_config(&config) {
            eprintln!("Error: {e}");
            return 1;
        }
        let config_path = client.scope_root().join("config.toml");
        if let Ok(content) = std::fs::read_to_string(&config_path) {
            let new_content = update_toml_key(&content, "relay_enabled", "true");
            if let Err(e) = crate::config::write_config_toml_path(&config_path, &new_content) {
                eprintln!("Error: Failed to write config: {e}");
                return 1;
            }
        }
        println!("Relay enabled\n");
        if !ensure_relay_worker_running_for_cli() {
            eprintln!("Error: relay daemon could not be started");
            return 1;
        }
        status_after_connect(client)
    } else {
        relay_off(client, &[])
    }
}

/// Persist relay settings to config.toml.
/// Relay auth is cleared when no password is provided so stale credentials
/// don't poison future joins/reconfigurations.
fn render_relay_config_content(content: &str, relay_id: &str, broker: &str, auth_token: Option<&str>, psk_b64: &str) -> String {
    let mut content = update_toml_key(content, "relay_id", &format!("\"{relay_id}\""));
    content = update_toml_key(&content, "relay", &format!("\"{broker}\""));
    content = update_toml_key(&content, "relay_enabled", "true");
    content = update_toml_key(&content, "relay_psk", &format!("\"{psk_b64}\""));
    update_toml_key(&content, "relay_token", &format!("\"{}\"", auth_token.unwrap_or("")))
}

fn persist_relay_config(client: &Client, relay_id: &str, broker: &str, auth_token: Option<&str>, psk: &[u8; 32]) -> Result<(), String> {
    let config_path = client.scope_root().join("config.toml");
    let content = std::fs::read_to_string(&config_path).unwrap_or_default();
    let psk_b64 = relay::encode_psk(psk);
    let content = render_relay_config_content(&content, relay_id, broker, auth_token, &psk_b64);
    crate::config::write_config_toml_path(&config_path, &content).map_err(|e| format!("Failed to write config: {e}"))
}

/// Create a new relay group.
fn relay_new(client: &Client, argv: &[String]) -> i32 {
    let (broker_url, auth_token, _) = parse_broker_flags(argv);

    // Ensure device_id file exists before spawning the daemon worker,
    // so both CLI and daemon use the same UUID (avoids TOCTOU race).
    if relay::read_device_uuid_in(client.scope_root()).is_none() {
        eprintln!("Error: failed to create device_id file");
        return 1;
    }

    let config = config_for(client);

    // Show previous group token if switching
    if let Some(old_token) = current_join_token(&config) {
        println!("Current group: agentx relay connect {old_token}\n");
    }

    let relay_id = uuid::Uuid::new_v4().to_string();
    let psk = match relay::crypto::generate_psk() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Error: failed to generate relay key: {e}");
            return 1;
        }
    };
    let pinned_broker = if let Some(broker) = &broker_url {
        // Private broker — test connectivity
        if let Some((host, port, use_tls)) = relay::parse_broker_url(broker) {
            println!("Testing {host}:{port}...");
            match relay::broker::ping_mqtt_broker(&host, port, use_tls, auth_token.as_deref()) {
                Ok(ms) => {
                    println!("  {host}:{port} — {ms}ms");
                    broker.clone()
                }
                Err(err) => {
                    eprintln!("  {host}:{port} — {err}");
                    eprintln!("\nBroker connection failed. Check network and credentials.");
                    return 1;
                }
            }
        } else {
            eprintln!("Invalid broker URL: {broker}");
            return 1;
        }
    } else {
        // Public broker — test all in parallel
        println!("Testing brokers...");
        let results = relay::broker::test_brokers_parallel(DEFAULT_BROKERS, auth_token.as_deref());
        let mut best = None;
        for (host, port, probe) in &results {
            match probe {
                BrokerProbe::Reachable(ms) => {
                    println!("  {host}:{port} — {ms}ms");
                    if best.is_none() {
                        best = Some(format!("mqtts://{host}:{port}"));
                    }
                }
                BrokerProbe::Failed(err) => println!("  {host}:{port} — failed ({err})"),
                BrokerProbe::Skipped => println!("  {host}:{port} — skipped"),
            }
        }
        match best {
            Some(b) => b,
            None => {
                eprintln!("\nNo broker accepted an MQTT connection. Check your network.");
                eprintln!("Or use a private broker: agentx relay new --broker mqtts://host:port");
                return 1;
            }
        }
    };

    // Clear the previous group's device state only once the new group is
    // certain to replace it: a failed probe above leaves the old config live.
    if let Err(error) = relay::clear_relay_device_state_client(client) {
        eprintln!("Error: {error}");
        return 1;
    }

    // Save config
    if let Err(e) = persist_relay_config(client, &relay_id, &pinned_broker, auth_token.as_deref(), &psk) {
        eprintln!("Error: {e}");
        return 1;
    }

    // Generate join token (always v0x04 — includes relay PSK)
    if let Some(token) = encode_join_token(&relay_id, &pinned_broker, &psk) {
        println!("\nBroker: {pinned_broker}");
        println!("Key:    {} (XChaCha20-Poly1305)", relay::crypto::fingerprint(&psk));
        if auth_token.is_some() {
            println!("Password: set");
        }
        println!("\nOn other devices: agentx relay connect {token}");
        if auth_token.is_some() {
            println!("  (they will also need: --password <secret>)");
        }
    }

    start_worker_and_report(client)
}

/// Connect to relay — re-enable or join with token.
fn relay_connect(client: &Client, argv: &[String]) -> i32 {
    let (broker_url, auth_token, remaining) = parse_broker_flags(argv);

    // Ensure device_id file exists before spawning the daemon worker,
    // so both CLI and daemon use the same UUID (avoids TOCTOU race).
    if relay::read_device_uuid_in(client.scope_root()).is_none() {
        eprintln!("Error: failed to create device_id file");
        return 1;
    }

    let token_str = remaining.first().filter(|s| !s.starts_with("-")).cloned();

    if token_str.is_none() {
        // Re-enable mode
        let config = config_for(client);
        if config.relay_id.is_empty() {
            eprintln!("No relay configured.");
            eprintln!("Run: agentx relay new");
            return 1;
        }
        if let Err(e) = validate_existing_relay_config(&config) {
            eprintln!("Error: {e}");
            return 1;
        }
        if config.relay_enabled {
            println!("Relay already enabled.\n");
            if !ensure_relay_worker_running_for_cli() {
                eprintln!("Error: relay is enabled but the daemon could not be started");
                return 1;
            }
            return status_after_connect(client);
        }
        return relay_toggle(client, true);
    }

    let Some(DecodedToken { relay_id, broker_url: token_broker, psk }) = decode_join_token(&token_str.unwrap()) else {
        eprintln!("Invalid Relay join token. Copy a current token from the source device.");
        return 1;
    };

    let effective_broker = broker_url.unwrap_or(token_broker);

    // Test broker connectivity
    let ping_ms = relay::parse_broker_url(&effective_broker).and_then(|(host, port, use_tls)| ping_broker(&host, port, use_tls));

    let config = config_for(client);

    // Switching groups: say so, with the way back, before dropping the old peers.
    if config.relay_id != relay_id {
        if let Some(old_token) = current_join_token(&config) {
            let old_key = relay::load_psk(&config).map(|psk| relay::crypto::fingerprint(&psk)).unwrap_or_default();
            println!("Leaving group {old_key}. To rejoin: agentx relay connect {old_token}\n");
        }
        if let Err(error) = relay::clear_relay_device_state_client(client) {
            eprintln!("Error: {error}");
            return 1;
        }
    }

    // Save config
    if let Err(e) = persist_relay_config(client, &relay_id, &effective_broker, auth_token.as_deref(), &psk) {
        eprintln!("Error: {e}");
        return 1;
    }

    if let Some(ms) = ping_ms {
        println!("Broker: {effective_broker} ({ms}ms)");
    } else {
        println!("Broker: {effective_broker}");
        eprintln!("  Warning: broker unreachable — check network or token");
    }
    println!("Key:    {}", relay::crypto::fingerprint(&psk));

    if auth_token.is_some() {
        println!("Password: set");
    } else {
        if !is_public_broker(&effective_broker) {
            println!("Password: not set (use --password if broker requires auth)");
        }
    }

    start_worker_and_report(client)
}

/// Update or add a key in TOML content (simple line-level editing).
/// Map flat relay field names to TOML section paths under [relay].
fn relay_toml_key(field: &str) -> (&str, &str) {
    match field {
        "relay" => ("relay", "url"),
        "relay_id" => ("relay", "id"),
        "relay_token" => ("relay", "token"),
        "relay_psk" => ("relay", "psk"),
        "relay_enabled" => ("relay", "enabled"),
        _ => panic!("unknown relay field: {field}"),
    }
}

/// Update a relay config field in config.toml using toml_edit for proper section handling.
fn update_toml_key(content: &str, field: &str, value: &str) -> String {
    let (section, key) = relay_toml_key(field);

    let mut doc = content.parse::<toml_edit::DocumentMut>().unwrap_or_else(|_| toml_edit::DocumentMut::new());

    // Ensure section exists
    if !doc.contains_table(section) {
        doc[section] = toml_edit::Item::Table(toml_edit::Table::new());
    }

    // Parse the value appropriately
    if value == "true" || value == "false" {
        doc[section][key] = toml_edit::value(value == "true");
    } else if let Some(stripped) = value.strip_prefix('"').and_then(|s| s.strip_suffix('"')) {
        doc[section][key] = toml_edit::value(stripped);
    } else {
        doc[section][key] = toml_edit::value(value);
    }

    doc.to_string()
}

pub(crate) fn try_local(args: &RelayArgs) -> Option<i32> {
    let argv = &args.args;
    match argv.first().map(String::as_str) {
        Some("token") => Some(relay_show_token()),
        Some("daemon") => Some(crate::cli::commands::daemon::cmd_daemon(&argv[1..])),
        Some("--help" | "-h") => {
            crate::cli::commands::help::print_command_help("relay");
            Some(0)
        }
        _ => None,
    }
}

pub(crate) fn cmd_relay(client: &Client, args: &RelayArgs) -> i32 {
    // --iam already stripped by router's extract_global_flags_full()
    let argv = &args.args;

    if argv.is_empty() {
        return relay_status(client);
    }

    let first = argv[0].as_str();

    if first == "--help" || first == "-h" {
        crate::cli::commands::help::print_command_help("relay");
        return 0;
    }

    match first {
        "new" => relay_new(client, &argv[1..]),
        "connect" => relay_connect(client, &argv[1..]),
        "off" | "disconnect" => relay_off(client, &argv[1..]),
        "on" => relay_connect(client, &Vec::new()),
        "status" => relay_status(client),
        "token" => relay_show_token(),
        "push" => relay_push(client),
        "daemon" => crate::cli::commands::daemon::cmd_daemon(&argv[1..]),
        _ => {
            // Could be a token passed directly
            if argv[0].len() > 20 && !argv[0].starts_with('-') { relay_connect(client, argv) } else { crate::cli::commands::help::unknown_subcommand("relay", first, &["new", "token", "connect", "on", "off", "daemon"]) }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrations::hooks::test_helpers::isolated_test_env;
    use serial_test::serial;

    fn native_client(scope: &std::path::Path) -> (crate::daemon::test_support::TestDaemon, Client) {
        let daemon = crate::daemon::test_support::TestDaemon::start(scope, &scope.join("agentx.db"));
        let client = Client::connect_existing(scope, crate::service::InvocationContext::capture().unwrap()).unwrap();
        (daemon, client)
    }

    fn fake_psk() -> [u8; 32] {
        [0x44; 32]
    }

    #[test]
    fn test_encode_decode_public_broker_token() {
        let relay_id = uuid::Uuid::new_v4().to_string();
        let broker = format!("mqtts://{}:{}", DEFAULT_BROKERS[0].0, DEFAULT_BROKERS[0].1);
        let psk = fake_psk();
        let token = encode_join_token(&relay_id, &broker, &psk).unwrap();
        let decoded = decode_join_token(&token).unwrap();
        assert_eq!(decoded.relay_id, relay_id);
        assert_eq!(decoded.broker_url, broker);
        assert_eq!(decoded.psk, psk);
    }

    #[test]
    fn test_encode_decode_private_broker_token() {
        let relay_id = uuid::Uuid::new_v4().to_string();
        let broker = "mqtts://my-broker.example.com:8883";
        let psk = fake_psk();
        let token = encode_join_token(&relay_id, broker, &psk).unwrap();
        let decoded = decode_join_token(&token).unwrap();
        assert_eq!(decoded.relay_id, relay_id);
        assert_eq!(decoded.broker_url, broker);
        assert_eq!(decoded.psk, psk);
    }

    #[test]
    fn test_decode_invalid_token() {
        assert!(decode_join_token("not-a-token").is_none());
        assert!(decode_join_token("").is_none());
    }

    #[test]
    fn test_update_toml_key_existing() {
        let content = "[relay]\nurl = \"\"\nid = \"\"\nenabled = false\n[other]\nfoo = 1\n";
        let result = update_toml_key(content, "relay_enabled", "true");
        assert!(result.contains("enabled = true"));
        assert!(result.contains("foo = 1"));
    }

    #[test]
    fn test_update_toml_key_new() {
        let content = "[other]\nfoo = 1\n";
        let result = update_toml_key(content, "relay_enabled", "true");
        // Should create [relay] section with enabled = true
        let doc: toml_edit::DocumentMut = result.parse().unwrap();
        assert_eq!(doc["relay"]["enabled"].as_bool(), Some(true));
        assert_eq!(doc["other"]["foo"].as_integer(), Some(1));
    }

    #[test]
    fn test_parse_broker_flags() {
        let argv: Vec<String> = vec!["--broker".into(), "mqtts://host:8883".into(), "--password".into(), "secret".into(), "other".into()];
        let (broker, auth, remaining) = parse_broker_flags(&argv);
        assert_eq!(broker.as_deref(), Some("mqtts://host:8883"));
        assert_eq!(auth.as_deref(), Some("secret"));
        assert_eq!(remaining, vec!["other"]);
    }

    #[test]
    #[serial]
    fn test_persist_relay_config_clears_stale_token_when_password_omitted() {
        let _ = isolated_test_env();
        let psk_b64 = relay::encode_psk(&fake_psk());
        let contents = render_relay_config_content("[relay]\nurl = \"mqtt://old:1883\"\nid = \"old-id\"\ntoken = \"stale-secret\"\npsk = \"old-psk\"\nenabled = true\n", "new-id", "mqtt://127.0.0.1:1", None, &psk_b64);
        let doc: toml_edit::DocumentMut = contents.parse().unwrap();
        assert_eq!(doc["relay"]["id"].as_str(), Some("new-id"));
        assert_eq!(doc["relay"]["url"].as_str(), Some("mqtt://127.0.0.1:1"));
        assert_eq!(doc["relay"]["token"].as_str(), Some(""));
        assert_eq!(doc["relay"]["psk"].as_str(), Some(psk_b64.as_str()));
        assert_eq!(doc["relay"]["enabled"].as_bool(), Some(true));
    }

    #[test]
    #[serial]
    fn test_relay_off_all_disables_local_relay_without_peers() {
        let (_dir, agentx_dir, _home, _guard) = isolated_test_env();
        let cfg = crate::config::AgentxConfig { relay: "mqtt://127.0.0.1:1".to_string(), relay_id: "relay-1".to_string(), relay_psk: relay::encode_psk(&fake_psk()), relay_enabled: true, ..Default::default() };
        crate::config::save_toml_config(&cfg).unwrap();

        let (_daemon, client) = native_client(&agentx_dir);
        let args = RelayArgs { args: vec!["off".to_string(), "--all".to_string()] };
        assert_eq!(cmd_relay(&client, &args), 0);

        let updated = crate::config::AgentxConfig::load(None);
        assert!(!updated.relay_enabled);
    }

    #[test]
    #[serial]
    fn test_relay_push_subcommand_exists() {
        let (_dir, agentx_dir, _home, _guard) = isolated_test_env();
        let (_daemon, client) = native_client(&agentx_dir);
        let args = RelayArgs { args: vec!["push".to_string()] };
        assert_eq!(cmd_relay(&client, &args), 0);
    }

    #[test]
    #[serial]
    fn test_relay_on_rejects_invalid_stored_config_without_enabling() {
        let (_dir, agentx_dir, _home, _guard) = isolated_test_env();
        let mut content = String::new();
        content = update_toml_key(&content, "relay_id", "\"relay-1\"");
        content = update_toml_key(&content, "relay", "\"\"");
        content = update_toml_key(&content, "relay_psk", "\"not-valid-base64\"");
        content = update_toml_key(&content, "relay_enabled", "false");
        std::fs::write(crate::paths::config_toml_path(), content).unwrap();

        let (_daemon, client) = native_client(&agentx_dir);
        let args = RelayArgs { args: vec!["on".to_string()] };
        assert_eq!(cmd_relay(&client, &args), 1);

        let updated = crate::config::AgentxConfig::load(None);
        assert!(!updated.relay_enabled);
    }

    #[test]
    #[serial]
    fn relay_status_transport_failure_does_not_recreate_database() {
        let (_dir, agentx_dir, _home, _guard) = isolated_test_env();
        let config = crate::config::AgentxConfig { relay: "mqtt://127.0.0.1:1".into(), relay_id: "fixture".into(), relay_enabled: true, relay_psk: relay::encode_psk(&fake_psk()), ..Default::default() };
        crate::config::save_toml_config(&config).unwrap();
        let (daemon, client) = native_client(&agentx_dir);
        drop(daemon);
        let database = agentx_dir.join("agentx.db");
        std::fs::remove_file(&database).unwrap();
        assert_eq!(relay_status(&client), 1);
        assert!(!database.exists());
    }
}
