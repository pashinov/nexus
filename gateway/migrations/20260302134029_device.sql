CREATE TABLE devices
(
    id         UUID PRIMARY KEY,
    uptime     BIGINT      NOT NULL,
    info       JSONB       NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE user_devices
(
    user_id   UUID NOT NULL REFERENCES users (id)   ON DELETE CASCADE,
    device_id UUID NOT NULL REFERENCES devices (id) ON DELETE CASCADE,
    PRIMARY KEY (device_id)
);
