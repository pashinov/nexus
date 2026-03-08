use futures_util::StreamExt;
use rdkafka::Message;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use serde::Deserialize;
use uuid::Uuid;

pub use config::KafkaConfig;

use crate::sqlx::SqlxClient;

mod config;

#[derive(Deserialize)]
struct DevicePayload {
    uptime: i64,
    #[serde(flatten)]
    info: serde_json::Value,
}

pub async fn run_consumer(config: KafkaConfig, db: SqlxClient) -> anyhow::Result<()> {
    let consumer: StreamConsumer = rdkafka::config::ClientConfig::new()
        .set("bootstrap.servers", &config.brokers)
        .set("group.id", &config.group_id)
        .set("client.id", &config.client_id)
        .set("enable.auto.commit", "false")
        .create()?;

    consumer.subscribe(&[config.topic.as_str()])?;
    tracing::info!(topic = %config.topic, "Kafka consumer started");

    let mut stream = consumer.stream();

    while let Some(result) = stream.next().await {
        let msg = match result {
            Err(e) => {
                tracing::error!("Kafka consumer error: {e}");
                continue;
            }
            Ok(msg) => msg,
        };

        let key = msg
            .key()
            .and_then(|k| std::str::from_utf8(k).ok())
            .and_then(|s| s.parse::<Uuid>().ok());

        let payload = msg
            .payload()
            .and_then(|p| std::str::from_utf8(p).ok())
            .and_then(|s| serde_json::from_str::<DevicePayload>(s).ok());

        match (key, payload) {
            (Some(id), Some(payload)) => {
                if let Err(e) = db.upsert_device(id, payload.uptime, payload.info).await {
                    tracing::error!("failed to upsert device {id}: {e:#}");
                }
            }
            _ => {
                tracing::warn!("invalid Kafka message, skipping");
            }
        }

        consumer.commit_message(&msg, CommitMode::Async).ok();
    }

    Ok(())
}
