use jotta_osd::{jotta::auth::TokenStore, Context};

pub mod config;
pub mod errors;
pub mod routes;
pub mod upload;

pub type AppResult<T> = Result<T, errors::AppError>;

pub type AppContext = Context<Box<dyn TokenStore>>;

/// Upload limit (1 GiB).
pub const UPLOAD_LIMIT: usize = 1 << 30;

#[macro_export]
macro_rules! create_app {
    ($jotta_config:expr, $ctx:expr) => {{
        use ::actix_web::{
            middleware,
            web::{Data, PayloadConfig},
            App,
        };
        use ::jotta_rest::routes;

        App::new()
            .app_data(Data::new($jotta_config.clone()))
            .app_data($ctx.clone())
            .app_data(PayloadConfig::new(::jotta_rest::UPLOAD_LIMIT))
            .wrap(middleware::NormalizePath::trim())
            .wrap(middleware::Logger::default())
            .configure(routes::config)
    }};
}
