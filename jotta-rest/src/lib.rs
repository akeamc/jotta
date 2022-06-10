use jotta_osd::{jotta::auth::Provider, Context};

pub mod config;
pub mod errors;
pub mod routes;

pub type AppResult<T> = Result<T, errors::AppError>;

pub type AppContext = Context<Box<dyn Provider>>;

#[macro_export]
macro_rules! create_app {
    ($jotta_config:expr, $ctx:expr) => {{
        use ::actix_web::{middleware, web::Data, App};
        use ::jotta_rest::routes;

        App::new()
            .app_data(Data::new($jotta_config.clone()))
            .app_data($ctx.clone())
            .wrap(middleware::NormalizePath::trim())
            .wrap(middleware::Logger::default())
            .configure(routes::config)
    }};
}
