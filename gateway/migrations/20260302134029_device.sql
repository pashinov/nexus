CREATE TABLE devices
(
    id             UUID PRIMARY KEY,
    client_version TEXT        NOT NULL,
    last_seen_at   TIMESTAMPTZ NOT NULL
);

CREATE TABLE user_devices
(
    user_id   UUID NOT NULL REFERENCES users (id)   ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices (id) ON DELETE CASCADE,
    PRIMARY KEY (device_id)
);
