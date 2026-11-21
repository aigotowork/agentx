//! Scalar label caller grammar and presentation over the existing exact management boundary.
use crate::management::contract::{AgentRef, Command, LabelsPatch, Request, Response};
use crate::service::{Request as NativeRequest, Response as NativeResponse};
use clap::{CommandFactory, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(name = "labels", about = "Read or atomically patch current Agent labels", after_help = "Values are scalar strings. Quote shell characters. On uncertain writes retain request_id and inspect labels receipt before issuing another patch.")]
pub(crate) struct Arguments {
    #[command(subcommand)]
    action: Action,
    /// Emit native structured result.
    #[arg(long, global = true)]
    pub json: bool,
}
#[derive(Debug, Subcommand)]
enum Action {
    /// Read labels and their opaque revision from the exact current Agent.
    Get { target: String },
    /// Atomically set key=value pairs; all unmentioned keys are preserved.
    Set {
        target: String,
        #[arg(required = true)]
        values: Vec<String>,
        #[arg(long)]
        if_revision: Option<String>,
        /// Stable UUID retained for reconciliation; omitted creates a fresh ID.
        #[arg(long)]
        request_id: Option<String>,
    },
    /// Atomically remove named keys; missing keys are a successful no-op.
    Remove {
        target: String,
        #[arg(required = true)]
        keys: Vec<String>,
        #[arg(long)]
        if_revision: Option<String>,
        #[arg(long)]
        request_id: Option<String>,
    },
    /// Inspect the durable result of an uncertain patch in this data generation.
    Receipt { request_id: String },
}
pub(crate) fn help_command() -> clap::Command {
    Arguments::command()
}

/// Reject duplicate set keys before a map can erase them. Native validation owns values.
pub(crate) fn parse_set(values: &[String]) -> Result<std::collections::BTreeMap<String, String>, String> {
    crate::agents::labels::parse_assignments(values).map_err(|error| error.to_string())
}
fn call(session: &crate::cli::client::Session, request: Request) -> Result<Response, String> {
    match session.client.call(NativeRequest::Management(request)).map_err(|e| e.to_string())? {
        NativeResponse::Management(response) => Ok(response),
        _ => Err("daemon does not support labels; use a compatible labels candidate".into()),
    }
}
fn target(session: &crate::cli::client::Session, name: &str) -> Result<AgentRef, String> {
    let name = if name == "self" { session.actor.as_ref().map(|actor| actor.participant.canonical_name.as_str()).ok_or("self requires a current caller identity")? } else { name };
    let record = session.records().map_err(|e| e.to_string())?.into_iter().find(|record| record.name == name && record.origin_device_id.is_none()).ok_or_else(|| format!("current local Agent not found: {name}"))?;
    Ok(AgentRef { name: record.name, incarnation: Some(record.created_at.to_bits().to_string()), stopped_event: None })
}
pub(crate) fn run(session: &crate::cli::client::Session, args: &Arguments) -> i32 {
    let result = execute(session, args);
    match result {
        Ok(value) => {
            println!("{}", serde_json::to_string(&value).expect("serializable labels result"));
            0
        }
        Err(error) => {
            eprintln!("Error: {error}");
            1
        }
    }
}
fn execute(session: &crate::cli::client::Session, args: &Arguments) -> Result<serde_json::Value, String> {
    let (name, set, remove, revision, request_id) = match &args.action {
        Action::Receipt { request_id } => {
            let Response::Operation(operation) = call(session, Request::Operation { request_id: request_id.clone() })? else { return Err("unexpected receipt response".into()) };
            return serde_json::to_value(operation).map_err(|e| e.to_string());
        }
        Action::Get { target: name } => {
            let agent = target(session, name)?;
            let Response::Labels(snapshot) = call(session, Request::Labels(agent))? else { return Err("unexpected labels read response".into()) };
            return serde_json::to_value(snapshot).map_err(|e| e.to_string());
        }
        Action::Set { target, values, if_revision, request_id } => (target, parse_set(values)?, Vec::new(), if_revision.clone(), request_id.clone()),
        Action::Remove { target, keys, if_revision, request_id } => (target, Default::default(), keys.clone(), if_revision.clone(), request_id.clone()),
    };
    let agent = target(session, name)?;
    let request_id = request_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
    let request = Request::Submit { request_id: request_id.clone(), command: Command::PatchLabels(LabelsPatch { agent, set, remove, if_revision: revision }), admit: true };
    let response = call(session, request).map_err(|error| format!("{error}; request_id={request_id}; reconcile with agentx labels receipt {request_id} --json"))?;
    let Response::Grant(grant) = response else { return Err("unexpected labels patch response".into()) };
    serde_json::to_value(grant.operation).map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scalar_values_and_duplicate_inputs() {
        assert_eq!(parse_set(&["note=a=b,c".into()]).unwrap()["note"], "a=b,c");
        assert!(parse_set(&["project=a".into(), "project=b".into()]).is_err());
        assert!(parse_set(&["project".into()]).is_err());
    }
}
