use uuid::Uuid;

use crate::sqlx::SqlxClient;

impl SqlxClient {
    /// Insert or update a device. Called on telemetry webhook.
    pub async fn upsert_device(&self, id: Uuid, client_version: &str) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO devices (id, client_version, last_seen_at)
            VALUES ($1, $2, now())
            ON CONFLICT (id) DO UPDATE
                SET client_version = EXCLUDED.client_version,
                    last_seen_at   = now()
            "#,
            id,
            client_version,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Bind a device to a user. Fails if device does not exist.
    pub async fn bind_device(&self, user_id: Uuid, device_id: Uuid) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO user_devices (user_id, device_id)
            VALUES ($1, $2)
            ON CONFLICT (device_id) DO UPDATE
                SET user_id = EXCLUDED.user_id
            "#,
            user_id,
            device_id,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
