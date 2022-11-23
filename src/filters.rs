use teloxide::types::Message;

use crate::bot::ConfigParameters;

// Returns true if incoming message is from a Trusted or Admin user
pub async fn is_trusted(cfg: ConfigParameters, msg: Message) -> bool {
    msg.from()
        .map(|user| {
            cfg.trusted_user_ids.iter().any(|&i| i == user.id)
                || cfg.admin_user_ids.iter().any(|&i| i == user.id)
        })
        .unwrap_or_default()
}

// Returns true if incoming message is from an Admin user
pub async fn is_admin(cfg: ConfigParameters, msg: Message) -> bool {
    msg.from()
        .map(|user| cfg.admin_user_ids.iter().any(|&i| i == user.id))
        .unwrap_or_default()
}

// Returns true if incoming message starts with "http"
pub async fn is_link(msg: Message) -> bool {
    let incoming_text = msg.text().unwrap_or_default();
    if incoming_text.starts_with("http") {
        return true;
    } else {
        return false;
    }
}
