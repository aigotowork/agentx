//! Passive diagnostic CLI. See `observe/AGENTS.md` for contracts and ownership.

use std::io::{self, Write};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use clap::{CommandFactory, Parser, error::ErrorKind};

mod args;
mod client;
mod output;
mod trace;

#[cfg(test)]
mod tests;

/// Parse and execute only the observation family, without ordinary CLI setup.
pub(crate) fn cmd_observe(argv: &[String]) -> i32 {
    let args = match args::ObserveArgs::try_parse_from(std::iter::once("agentx observe".to_owned()).chain(argv.iter().cloned())) {
        Ok(args) => args,
        Err(error) => {
            let help = matches!(error.kind(), ErrorKind::DisplayHelp | ErrorKind::DisplayVersion);
            if help {
                return write_help(&error.to_string());
            }
            let _ = write!(io::stderr().lock(), "{error}");
            return 2;
        }
    };
    let command = match args.prepare() {
        Ok(command) => command,
        Err(error) => return usage_error(&error),
    };
    if let args::PreparedCommand::Trace { id, local, limit } = command {
        return match trace::read(args.scope, id, local, limit, args.json, &mut io::stdout().lock()) {
            Ok(()) => 0,
            Err(error) => output_error(error),
        };
    }
    let interrupted = Arc::new(AtomicBool::new(false));
    crate::sys::signal::register_int(&interrupted);
    let client = match client::ObservationClient::capture(&args, &command) {
        Ok(client) => client,
        Err(record) => return emit_once(&record, args.json),
    };
    if let Err(error) = client.validate_scope(&command) {
        return usage_error(&error);
    }
    if interrupted.load(Ordering::Relaxed) {
        return 130;
    }
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    let mut record = client.read(&command);
    loop {
        if let Err(error) = output::write_record(&mut writer, &record, args.json) {
            return output_error(error);
        }
        if interrupted.load(Ordering::Relaxed) {
            return 130;
        }
        let args::PreparedCommand::Events { follow: true, limit, .. } = &command else {
            return record.exit_code();
        };
        if let Some(code) = record.follow_exit_code() {
            if record.exit_code() == 0 {
                let _ = writeln!(io::stderr().lock(), "Observation stopped at a gap or era boundary. Preserve the result and explicitly select a new event window.");
            }
            return code;
        }
        if !wait_for_poll(&interrupted) {
            return 130;
        }
        record = client.follow_page(&record, *limit);
    }
}

/// Derive help from the same clap declarations used for execution.
pub(crate) fn help_text(topic: &[&str]) -> String {
    let mut root = help_command();
    root.build();
    let mut selected = &mut root;
    for name in topic {
        if selected.find_subcommand(name).is_none() {
            break;
        }
        selected = selected.find_subcommand_mut(name).expect("checked clap subcommand");
    }
    selected.render_long_help().to_string()
}

pub(crate) fn help_command() -> clap::Command {
    args::ObserveArgs::command()
}

fn write_help(text: &str) -> i32 {
    let text = format!("{}\n", crate::cli::commands::help::compact_help(text));
    match io::stdout().lock().write_all(text.as_bytes()) {
        Ok(()) => 0,
        Err(error) => output_error(error),
    }
}

fn usage_error(message: &str) -> i32 {
    let _ = writeln!(io::stderr().lock(), "agentx observe: {message}\nUse agentx observe --help for command syntax.");
    2
}

fn emit_once(record: &output::ObserveOutput, json: bool) -> i32 {
    match output::write_record(&mut io::stdout().lock(), record, json) {
        Ok(()) => record.exit_code(),
        Err(error) => output_error(error),
    }
}

fn output_error(error: io::Error) -> i32 {
    if error.kind() == io::ErrorKind::BrokenPipe {
        return 0;
    }
    let _ = writeln!(io::stderr().lock(), "agentx observe: could not write diagnostic output ({:?})", error.kind());
    1
}

/// No subscription/server pin exists. This caller-owned timer checks SIGINT
/// frequently, and the native read itself has a separate one-second deadline.
fn wait_for_poll(interrupted: &AtomicBool) -> bool {
    let deadline = Instant::now() + Duration::from_secs(1);
    while !interrupted.load(Ordering::Relaxed) {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return true;
        };
        std::thread::sleep(remaining.min(Duration::from_millis(25)));
    }
    false
}
