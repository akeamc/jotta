pub mod errors;
pub mod routes;

#[derive(Debug)]
pub struct AppConfig {
    pub connections_per_transfer: usize,
}

pub(crate) type AppResult<T> = Result<T, errors::AppError>;
