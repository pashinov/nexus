use rumqttc::{Event, Packet};

use crate::config::AppConfig;
use crate::mqtt::MqttClient;
use crate::storage::Storage;

pub async fn mqtt_service(config: AppConfig) -> anyhow::Result<()> {
    let storage = Storage::open(&config.storage)?;

    tracing::info!(host = %config.mqtt.host, port = config.mqtt.port, "connecting to MQTT broker");

    let (mqtt_client, mut eventloop) = MqttClient::builder()
        .with_config(config.mqtt)
        .with_storage(storage)
        .build()?;

    let topic = format!("devices/{}/#", mqtt_client.client_id());
    mqtt_client.subscribe(&topic).await?;
    tracing::info!(%topic, "subscribed");

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            let payload = serde_json::json!({ "version": VERSION }).to_string();
            if let Err(e) = mqtt_client
                .publish(
                    &format!("devices/{}/info", mqtt_client.client_id()),
                    payload.as_bytes(),
                )
                .await
            {
                tracing::error!("failed to publish version: {e:#}");
            }
        }
    });

    tokio::spawn(async move {
        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => {
                    tracing::info!("connected to MQTT broker");
                }
                Ok(Event::Incoming(Packet::Publish(publish))) => {
                    tracing::info!(
                        topic = %publish.topic,
                        bytes = publish.payload.len(),
                        "received message"
                    );

                    // TODO: handle CTRL commands
                }
                Ok(Event::Incoming(Packet::Disconnect)) => {
                    tracing::warn!("disconnected from MQTT broker");
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("MQTT connection error: {e:#}");
                }
            }
        }
    });

    Ok(())
}

static VERSION: &str = env!("MQTT_CLIENT_VERSION");
