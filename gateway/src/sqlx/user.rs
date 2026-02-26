use sqlx::PgPool;

use crate::sqlx::SqlxClient;

impl SqlxClient {
    pub fn new(pool: PgPool) -> SqlxClient {
        SqlxClient { pool }
    }

    /// Insert or update a user.
    /// Returns the user's internal UUID.
    pub async fn upsert_user(
        &self,
        sub: &str,
        email: &str,
        name: &str,
    ) -> anyhow::Result<uuid::Uuid> {
        let id = sqlx::query_scalar!(
            r#"
            INSERT INTO users (sub, email, name)
            VALUES ($1, $2, $3)
            ON CONFLICT (sub) DO UPDATE
                SET email      = EXCLUDED.email,
                    name       = EXCLUDED.name,
                    updated_at = now()
            RETURNING id
            "#,
            sub,
            email,
            name,
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(id)
    }
}
