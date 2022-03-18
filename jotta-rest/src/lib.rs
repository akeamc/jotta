pub mod errors;
pub mod routes;

pub(crate) type AppResult<T> = Result<T, errors::AppError>;
