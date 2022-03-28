use jotta_osd::{jotta::auth::TokenStore, Context};

pub mod errors;
pub mod routes;
pub mod settings;

pub type AppResult<T> = Result<T, errors::AppError>;

pub type AppContext = Context<Box<dyn TokenStore>>;
