# Nexus

IoT platform for device management and monitoring.

## Services

| Service  | Description                                      | Path                            |
|----------|--------------------------------------------------|---------------------------------|
| gateway  | HTTP API gateway for authentication and devices  | [`gateway/`](gateway/)          |
| postgres | PostgreSQL 16 database                           | [`infrastructure/postgres/`](infrastructure/postgres/) |
| emqx     | MQTT broker for device communication             | [`infrastructure/emqx/`](infrastructure/emqx/)         |
