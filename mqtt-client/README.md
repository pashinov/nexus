# mqtt-client

MQTT client for Nexus IoT devices. Connects to EMQX broker via mTLS, subscribes to device-specific topics and publishes
telemetry.

## Build

```bash
cargo build --release -p mqtt-client
```

## Run

```bash
mqtt-client run -c config.json
```

## Certificates

See [`infrastructure/emqx/certs/`](../infrastructure/emqx/certs/) for certificate generation.

## Configuration

Built-in defaults are used when no file is provided. Pass `--config config.json` to override:

```json
{
  "mqtt": {
    "host": "localhost",
    "port": 8883,
    "keep_alive_secs": 60,
    "channel_capacity": 10,
    "ca_cert": "/etc/mqtt-client/certs/ca.pem",
    "client_cert": "/etc/mqtt-client/certs/client.pem",
    "client_key": "/etc/mqtt-client/certs/client.key"
  },
  "storage": {
    "db_path": "./db/client.db"
  }
}
```
