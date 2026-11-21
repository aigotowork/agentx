//! Pure message and product guidance shared by daemon formatting and clients.

/// Centralized tip text.
pub fn get_tip(key: &str) -> Option<&'static str> {
    match key {
        "list:status" => Some(
            "[tip] Statuses: \u{25b6} active (will read new msgs very soon)  \u{25c9} listening (will read new msgs in <1s)  \
             \u{25a0} blocked (needs human user approval)  \u{25cb} inactive (dead)  \u{25e6} unknown (neutral)",
        ),
        "list:types" => Some("[tip] Tool labels: [CLAUDE] etc. get messages automatically; a * means not bound yet or lost (agentx list <name>); [AD-HOC] checks manually with agentx listen."),
        // Send-side
        "send:intent:request" => Some("[tip] intent=request: You signaled you expect a response. You'll be auto-notified if they end their turn or stop without responding. Safe to move on."),
        "send:intent:inform" => Some("[tip] intent=inform: You signaled no response needed."),
        "send:intent:ack" => Some("[tip] intent=ack: You acknowledged receipt. Recipient won't respond."),
        // Recv-side
        "recv:intent:request" => Some("[tip] intent=request: Sender expects a response."),
        "recv:intent:inform" => Some("[tip] intent=inform: Sender doesn't expect a response."),
        "recv:intent:ack" => Some("[tip] intent=ack: Sender confirmed receipt. No response needed."),
        // @mention matching
        "mention:matching" => Some("[tip] @targets: @luna matches one exact local agent | @luna:BOXE matches a remote agent; a unique device prefix is accepted"),
        // Subscriptions
        "sub:created" => Some("[tip] You'll be notified via agentx message when the next matching event occurs. Safe to end your turn."),
        _ => None,
    }
}
