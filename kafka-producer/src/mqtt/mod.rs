use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS, Transport};

pub use self::config::MqttConfig;
use crate::storage::Storage;

mod config;

pub struct MqttClientBuilder<MandatoryFields = (Storage,)> {
    config: MqttConfig,
    mandatory_fields: MandatoryFields,
}

impl MqttClientBuilder {
    pub fn build(self) -> anyhow::Result<(MqttClient, EventLoop)> {
        let (storage,) = self.mandatory_fields;
        let config = &self.config;

        let client_id = storage.client_id()?;

        let ca = std::fs::read(&config.ca_cert)
            .with_context(|| format!("failed to read CA cert: {}", config.ca_cert.display()))?;
        let cert = std::fs::read(&config.client_cert).with_context(|| {
            format!(
                "failed to read client cert: {}",
                config.client_cert.display()
            )
        })?;
        let key = std::fs::read(&config.client_key).with_context(|| {
            format!("failed to read client key: {}", config.client_key.display())
        })?;

        let connector = native_tls::TlsConnector::builder()
            .add_root_certificate(
                native_tls::Certificate::from_pem(&ca).context("failed to parse CA cert")?,
            )
            .identity(
                native_tls::Identity::from_pkcs8(&cert, &key)
                    .context("failed to parse client identity")?,
            )
            .build()
            .context("failed to build TLS connector")?;

        let mut opts = MqttOptions::new(client_id.to_string(), &config.host, config.port);
        opts.set_clean_session(config.clean_session);
        opts.set_keep_alive(Duration::from_secs(config.keep_alive_secs as u64));
        opts.set_transport(Transport::tls_with_config(connector.into()));

        let (client, event_loop) = AsyncClient::new(opts, config.channel_capacity);

        Ok((
            MqttClient {
                inner: Arc::new(Inner { client }),
            },
            event_loop,
        ))
    }
}

impl MqttClientBuilder<((),)> {
    pub fn with_storage(self, storage: Storage) -> MqttClientBuilder<(Storage,)> {
        MqttClientBuilder {
            config: self.config,
            mandatory_fields: (storage,),
        }
    }
}

impl<T> MqttClientBuilder<(T,)> {
    pub fn with_config(self, config: MqttConfig) -> MqttClientBuilder<(T,)> {
        MqttClientBuilder { config, ..self }
    }
}

#[derive(Clone)]
pub struct MqttClient {
    inner: Arc<Inner>,
}

impl MqttClient {
    pub fn builder() -> MqttClientBuilder<((),)> {
        MqttClientBuilder {
            config: MqttConfig::default(),
            mandatory_fields: ((),),
        }
    }

    pub async fn subscribe(&self, topic: &str) -> anyhow::Result<()> {
        self.inner
            .client
            .subscribe(topic, QoS::AtLeastOnce)
            .await
            .context("failed to subscribe to topic")
    }
}

struct Inner {
    client: AsyncClient,
}
