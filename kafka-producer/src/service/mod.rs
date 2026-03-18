use rumqttc::{Event, Packet};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::config::AppConfig;
use crate::kafka::KafkaProducer;
use crate::mqtt::MqttClient;
use crate::storage::Storage;

pub async fn run_service(config: AppConfig, token: CancellationToken) -> anyhow::Result<()> {
    let producer = KafkaProducer::new(config.kafka)?;

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<(String, String)>();

    let storage = Storage::open(&config.storage)?;

    tracing::info!(host = %config.mqtt.host, port = config.mqtt.port, "MQTT broker connecting");

    let (mqtt_client, mut eventloop) = MqttClient::builder()
        .with_config(config.mqtt)
        .with_storage(storage)
        .build()?;

    mqtt_client.subscribe(&config.mqtt_topic).await?;

    // MQTT event loop → channel
    let mqtt = tokio::spawn({
        let token = token.clone();
        async move {
            loop {
                tokio::select! {
                    _ = token.cancelled() => break,
                    result = eventloop.poll() => match result {
                        Ok(Event::Incoming(Packet::Publish(p))) => {
                            if let Some(device_id) = extract_device_id(&p.topic) {
                                match String::from_utf8(p.payload.to_vec()) {
                                    Ok(payload) => {
                                        if tx.send((device_id.to_string(), payload)).is_err() {
                                            break;
                                        }
                                    }
                                    Err(err) => {
                                        tracing::warn!(?device_id, "invalid MQTT message payload: {err}");
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
                        Err(err) => {
                            tracing::error!("MQTT error: {err:#}");
                        }
                    }
                }
            }
            tracing::info!("MQTT eventloop stopped");
        }
    });

    // channel → Kafka producer
    let kafka = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = token.cancelled() => break,
                item = rx.recv() => {
                    match item {
                        Some((device_id, payload)) => {
                            producer.send(&device_id, &payload).await;
                        }
                        None => break,
                    }
                }
            }
        }
        tracing::info!("Kafka forwarder stopped");
    });

    let _ = tokio::join!(mqtt, kafka);

    tracing::info!("kafka-producer stopped");

    Ok(())
}

fn extract_device_id(topic: &str) -> Option<Uuid> {
    // topic format: device/{device_id}/...
    match topic.split('/').collect::<Vec<_>>().as_slice() {
        ["device", device_id, ..] => device_id.parse().ok(),
        _ => None,
    }
}
