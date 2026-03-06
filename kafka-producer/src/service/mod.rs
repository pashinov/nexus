use rumqttc::{Event, Packet};
use uuid::Uuid;

use crate::config::AppConfig;
use crate::kafka::KafkaProducer;
use crate::mqtt::MqttClient;
use crate::storage::Storage;

pub async fn run_service(config: AppConfig) -> anyhow::Result<()> {
    let producer = KafkaProducer::new(config.kafka)?;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(String, String)>();

    let storage = Storage::open(&config.storage)?;

    let (mqtt_client, mut eventloop) = MqttClient::builder()
        .with_config(config.mqtt)
        .with_storage(storage)
        .build()?;

    mqtt_client.subscribe(&config.mqtt_topic).await?;

    // MQTT event loop → channel
    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::Publish(p))) => {
                    if let Some(device_id) = extract_device_id(&p.topic) {
                        match String::from_utf8(p.payload.to_vec()) {
                            Ok(payload) => {
                                tx.send((device_id.to_string(), payload))
                                    .expect("should be alive");
                            }
                            Err(e) => {
                                tracing::warn!(?device_id, "invalid MQTT message payload: {e}");
                            }
                        }
                    }
                }
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    tracing::info!("MQTT broker connected");
                }
                Ok(Event::Incoming(Packet::Disconnect)) => {
                    tracing::warn!("MQTT broker disconnected");
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("MQTT error: {e:#}");
                }
            }
        }
    });

    // channel → Kafka producer
    tokio::spawn(async move {
        while let Some((device_id, payload)) = rx.recv().await {
            producer.send(&device_id, &payload).await;
        }
    });

    Ok(())
}

fn extract_device_id(topic: &str) -> Option<Uuid> {
    // topic format: devices/{device_id}/...
    match topic.split('/').collect::<Vec<_>>().as_slice() {
        ["devices", device_id, ..] => device_id.parse().ok(),
        _ => None,
    }
}
