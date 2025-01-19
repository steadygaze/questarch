#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::postgres::PgPool,
}
