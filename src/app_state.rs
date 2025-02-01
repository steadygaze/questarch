use leptos::prelude::*;
use lettre::{AsyncSmtpTransport, Tokio1Executor};

/// Easily cloneable prototype.
#[allow(dead_code)] // For prototyping
#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::postgres::PgPool,
    pub valkey_pool: fred::clients::Pool,
    pub mailer: AsyncSmtpTransport<Tokio1Executor>,
}

/// Wrapper to get AppState that's easily usable with the ? operator, for use in
/// server functions.
pub fn use_app_state() -> Result<AppState, ServerFnError> {
    match use_context::<AppState>() {
        Some(app_state) => Ok(app_state),
        None => Err(ServerFnError::new("Couldn't get AppState from context")),
    }
}
