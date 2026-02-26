use sqlx::PgPool;

pub use self::config::PgConfig;

mod config;
mod user;

#[derive(Clone)]
pub struct SqlxClient {
    pool: PgPool,
}
