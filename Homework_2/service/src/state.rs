use deadpool_postgres::Pool;

#[derive(Clone)]
pub struct Config {
    pub order_rate_limit_minutes: i64,
    pub jwt_secret: String,
}

#[derive(Clone)]
pub struct State {
    pub db: Pool,
    pub config: Config,
}
