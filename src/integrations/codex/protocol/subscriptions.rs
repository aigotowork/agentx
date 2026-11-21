//! Virtualize only notification opt-outs required for reliable internal input
//! observation. The outer client's identity and other capabilities stay native.

use std::collections::{HashMap, HashSet};

use super::{FrameKind, NativeFrame, ProtocolFailure, RawObject, RpcId};

const OBSERVED_NOTIFICATIONS: &[&str] = &["thread/started", "thread/status/changed", "thread/closed", "thread/archived", "thread/deleted", "turn/started", "turn/completed", "item/completed", "hook/completed", "serverRequest/resolved"];

/// Readiness requires the successful initialize response and an initialized
/// notification sent afterward. A rejected duplicate initialize cannot clear
/// a working handshake or silently replace its subscription choice.
#[derive(Default)]
pub(crate) struct ConnectionHandshake {
    pending: HashMap<RpcId, HashSet<String>>,
    outer_opt_out: HashSet<String>,
    accepted: bool,
    ready: bool,
}

impl ConnectionHandshake {
    pub(crate) fn prepare_initialize(&mut self, downstream_id: RpcId, raw: &str) -> Result<String, ProtocolFailure> {
        let message = NativeFrame::parse(raw)?;
        if !matches!(&message.kind, FrameKind::Request { method, .. } if method == "initialize") {
            return Ok(raw.to_owned());
        }
        let Some(mut params) = message.params()? else {
            self.pending.insert(downstream_id, HashSet::new());
            return Ok(raw.to_owned());
        };
        let Some(raw_capabilities) = params.get("capabilities") else {
            self.pending.insert(downstream_id, HashSet::new());
            return Ok(raw.to_owned());
        };
        if raw_capabilities.get() == "null" {
            self.pending.insert(downstream_id, HashSet::new());
            return Ok(raw.to_owned());
        }
        let mut capabilities = RawObject::parse(raw_capabilities.get())?;
        let Some(raw_methods) = capabilities.get("optOutNotificationMethods") else {
            self.pending.insert(downstream_id, HashSet::new());
            return Ok(raw.to_owned());
        };
        if raw_methods.get() == "null" {
            self.pending.insert(downstream_id, HashSet::new());
            return Ok(raw.to_owned());
        }
        let methods: Vec<String> = serde_json::from_str(raw_methods.get())?;
        self.pending.insert(downstream_id, methods.iter().cloned().collect());
        let native_methods: Vec<&str> = methods.iter().map(String::as_str).filter(|method| !OBSERVED_NOTIFICATIONS.contains(method)).collect();
        if native_methods.len() == methods.len() {
            return Ok(raw.to_owned());
        }
        capabilities.set("optOutNotificationMethods", &native_methods)?;
        params.set("capabilities", &capabilities)?;
        let mut envelope = message.object;
        envelope.set("params", &params)?;
        envelope.render()
    }

    pub(crate) fn observe_response(&mut self, downstream_id: &RpcId, raw: &str) -> Result<(), ProtocolFailure> {
        let Some(opt_out) = self.pending.remove(downstream_id) else {
            return Ok(());
        };
        let message = NativeFrame::parse(raw)?;
        if message.object.get("error").is_none() && !self.accepted {
            self.outer_opt_out = opt_out;
            self.accepted = true;
        }
        Ok(())
    }

    /// Call after forwarding initialized, rather than before a possibly failed
    /// native write. An early notification never grants internal input access.
    pub(crate) fn observe_initialized(&mut self) {
        if self.accepted {
            self.ready = true;
        }
    }

    pub(crate) fn ready(&self) -> bool {
        self.ready
    }

    pub(crate) fn forwards_notification(&self, method: &str) -> bool {
        !self.outer_opt_out.contains(method)
    }
}
