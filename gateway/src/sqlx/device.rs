use uuid::Uuid;

use crate::api::models::device::DeviceInfo;
use crate::sqlx::SqlxClient;

impl SqlxClient {
    /// Insert or update a device info.
    pub async fn upsert_device(
        &self,
        id: Uuid,
        uptime: i64,
        info: serde_json::Value,
    ) -> anyhow::Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO devices (id, uptime, info)
            VALUES ($1, $2, $3)
            ON CONFLICT (id) DO UPDATE
                SET uptime     = EXCLUDED.uptime,
                    info       = EXCLUDED.info,
                    updated_at = now()
            "#,
            id,
            uptime,
            info,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get all devices bound to a user.
    pub async fn get_user_devices(&self, user_id: Uuid) -> anyhow::Result<Vec<DeviceInfo>> {
        let rows = sqlx::query_as!(
            DeviceInfo,
            r#"
            SELECT devices.id, devices.uptime, devices.info, devices.created_at, devices.updated_at
            FROM devices
            JOIN user_devices ON user_devices.device_id = devices.id
            WHERE user_devices.user_id = $1
            ORDER BY devices.updated_at DESC
            "#,
            user_id,
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows)
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
