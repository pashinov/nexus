use rumqttc::{Event, Packet};
use tokio_util::sync::CancellationToken;

use crate::config::AppConfig;
use crate::mqtt::MqttClient;
use crate::storage::Storage;

mod command;
mod system;

pub async fn mqtt_service(config: AppConfig, token: CancellationToken) -> anyhow::Result<()> {
    let storage = Storage::open(&config.storage)?;

    tracing::info!(host = %config.mqtt.host, port = config.mqtt.port, "MQTT broker connecting");

    let (mqtt_client, mut eventloop) = MqttClient::builder()
        .with_config(config.mqtt)
        .with_storage(storage)
        .build()?;

    let pub_handler = tokio::spawn({
        let token = token.clone();
        let mqtt_client = mqtt_client.clone();

        async move {
            let mut interval = tokio::time::interval(config.publish_info_interval);
            loop {
                tokio::select! {
                    _ = interval.tick() => {}
                    _ = token.cancelled() => break,
                }

                let payload = system::device_info().await;

                if let Err(e) = mqtt_client
                    .publish(
                        &format!("device/{}/info", mqtt_client.client_id()),
                        payload.as_bytes(),
                    )
                    .await
                {
                    tracing::error!("failed to publish device info: {e:#}");
                }
            }
            tracing::info!("publish task stopped");
        }
    });

    let sub_handler = tokio::spawn({
        let topic = format!("device/{}/command", mqtt_client.client_id());
        mqtt_client.subscribe(&topic).await?;
        tracing::info!(%topic, "subscribed");

        async move {
            loop {
                tokio::select! {
                    result = eventloop.poll() => {
                        match result {
                            Ok(Event::Incoming(Packet::ConnAck(_))) => {
                                tracing::info!("MQTT broker connected");
                            }
                            Ok(Event::Incoming(Packet::Publish(publish))) => {
                                tracing::info!(
                                    topic = %publish.topic,
                                    bytes = publish.payload.len(),
                                    "received MQTT message"
                                );
                                command::handle(&mqtt_client, &mqtt_client.client_id().to_string(), &publish.payload).await;
                            }
                            Ok(Event::Incoming(Packet::Disconnect)) => {
                                tracing::warn!("MQTT broker disconnected");
                            }
                            Ok(_) => {}
                            Err(err) => {
                                tracing::error!("MQTT broker connection error: {err:#}");
                            }
                        }
                    }
                    _ = token.cancelled() => break,
                }
            }
            tracing::info!("eventloop task stopped");
        }
    });

    let _ = tokio::join!(pub_handler, sub_handler);

    tracing::info!("mqtt-client stopped");

    Ok(())
}
