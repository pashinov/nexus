# Nexus

IoT platform for device management and monitoring.

## Services

| Service      | Description                                     | Path                                                                   |
|--------------|-------------------------------------------------|------------------------------------------------------------------------|
| gateway      | HTTP API gateway for authentication and devices | [`gateway/`](gateway/)                                                 |
| mqtt-client  | MQTT client for IoT devices                     | [`mqtt-client/`](mqtt-client/)                                         |
| postgres     | PostgreSQL database                             | [`infrastructure/postgres/`](infrastructure/postgres/)                 |
| emqx         | MQTT broker for device communication            | [`infrastructure/emqx/`](infrastructure/emqx/)                         |
