use sqlx::PgPool;

pub use self::config::PgConfig;

mod config;
mod device;
mod user;

#[derive(Clone)]
pub struct SqlxClient {
    pool: PgPool,
}
