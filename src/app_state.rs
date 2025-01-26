use leptos::prelude::*;

/// Easily cloneable prototype.
#[allow(dead_code)] // For prototyping
#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::postgres::PgPool,
    pub valkey_pool: fred::clients::Pool,
}
