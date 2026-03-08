use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::mqtt::MqttClient;

#[derive(Deserialize)]
pub struct Command {
    pub id: Uuid,
    #[serde(flatten)]
    pub kind: CommandKind,
}

#[derive(Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum CommandKind {
    Ping,
}

#[derive(Serialize)]
struct Ack {
    id: Uuid,
    status: &'static str,
}

pub async fn handle(client: &MqttClient, device_id: &str, payload: &[u8]) {
    let cmd = match serde_json::from_slice::<Command>(payload) {
        Ok(cmd) => cmd,
        Err(e) => {
            tracing::warn!("failed to parse command: {e:#}");
            return;
        }
    };

    match cmd.kind {
        CommandKind::Ping => {
            tracing::info!(id = %cmd.id, "ping received");

            let ack = serde_json::to_string(&Ack {
                id: cmd.id,
                status: "ok",
            })
            .expect("shouldn't fail");

            let topic = format!("device/{device_id}/command/ack");
            if let Err(e) = client.publish(&topic, ack.as_bytes()).await {
                tracing::error!("failed to publish ack: {e:#}");
            }
        }
    }
}
