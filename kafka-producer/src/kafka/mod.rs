use std::time::Duration;

use rdkafka::producer::{FutureProducer, FutureRecord, Producer};

pub use config::KafkaConfig;

mod config;

pub struct KafkaProducer {
    config: KafkaConfig,
    producer: FutureProducer,
}

impl KafkaProducer {
    pub fn new(config: KafkaConfig) -> anyhow::Result<Self> {
        let mut client_config = rdkafka::config::ClientConfig::new();
        client_config.set("bootstrap.servers", &config.brokers);

        if let Some(message_timeout_ms) = config.message_timeout_ms {
            client_config.set("message.timeout.ms", message_timeout_ms.to_string());
        }
        if let Some(message_max_size) = config.message_max_size {
            client_config.set("message.max.bytes", message_max_size.to_string());
        }

        let producer = client_config.create()?;

        Ok(Self { config, producer })
    }

    /// Sends a message to Kafka, retrying indefinitely until delivered.
    pub async fn send(&self, key: &str, payload: &str) {
        let topic = &self.config.topic;
        let attempt_interval = Duration::from_millis(self.config.attempt_interval_ms);

        loop {
            let record = FutureRecord::to(topic).key(key).payload(payload);
            match self.producer.send_result(record) {
                Ok(future) => match future.await {
                    Ok(_) => return,
                    Err(e) => {
                        tracing::error!(key, "Kafka delivery error, retrying: {e:?}");
                        tokio::time::sleep(attempt_interval).await;
                    }
                },
                Err((e, _)) => {
                    tracing::error!(key, "Kafka send error, retrying: {e:?}");
                    tokio::time::sleep(attempt_interval).await;
                }
            }
        }
    }
}

impl Drop for KafkaProducer {
    fn drop(&mut self) {
        tracing::warn!("flushing kafka producer");
        self.producer.flush(None).ok();
    }
}
