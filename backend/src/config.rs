use std::{env, net::SocketAddr};

use tower_http::cors::AllowOrigin;

pub const DEFAULT_MODEL: &str = "nvidia/nemotron-3-super-120b-a12b:free";

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_addr: SocketAddr,
    pub frontend_origin: String,
    pub openrouter_api_key: Option<String>,
    pub openrouter_model: String,
}

impl Config {
    pub fn from_env() -> Result<Self, ConfigError> {
        let database_url =
            env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://gpt-copy-v5.sqlite3".to_owned());
        let host = env::var("BACKEND_HOST").unwrap_or_else(|_| "127.0.0.1".to_owned());
        let port = env::var("BACKEND_PORT")
            .unwrap_or_else(|_| "3001".to_owned())
            .parse::<u16>()
            .map_err(|source| ConfigError::InvalidPort { source })?;
        let bind_addr = format!("{host}:{port}")
            .parse::<SocketAddr>()
            .map_err(|source| ConfigError::InvalidBindAddress { source })?;
        let frontend_origin =
            env::var("FRONTEND_ORIGIN").unwrap_or_else(|_| "http://localhost:3000".to_owned());
        let openrouter_api_key = env::var("OPENROUTER_API_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty());
        let openrouter_model =
            env::var("OPENROUTER_MODEL").unwrap_or_else(|_| DEFAULT_MODEL.to_owned());

        Ok(Self {
            database_url,
            bind_addr,
            frontend_origin,
            openrouter_api_key,
            openrouter_model,
        })
    }

    pub fn cors_origin(&self) -> Result<AllowOrigin, ConfigError> {
        let header_value = self
            .frontend_origin
            .parse()
            .map_err(|source| ConfigError::InvalidFrontendOrigin { source })?;
        Ok(AllowOrigin::exact(header_value))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("BACKEND_PORT must be a valid u16")]
    InvalidPort { source: std::num::ParseIntError },
    #[error("BACKEND_HOST and BACKEND_PORT must form a valid socket address")]
    InvalidBindAddress { source: std::net::AddrParseError },
    #[error("FRONTEND_ORIGIN must be a valid header value")]
    InvalidFrontendOrigin {
        source: axum::http::header::InvalidHeaderValue,
    },
}
