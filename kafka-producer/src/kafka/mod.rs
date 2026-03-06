use rdkafka::producer::{FutureProducer, FutureRecord};

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

    pub async fn send(&self, key: &str, payload: &str) -> anyhow::Result<()> {
        let topic = &self.config.topic;
        self.producer
            .send(
                FutureRecord::to(topic).key(key).payload(payload),
                std::time::Duration::ZERO,
            )
            .await
            .map_err(|(e, _)| anyhow::anyhow!(e))?;
        Ok(())
    }
}
