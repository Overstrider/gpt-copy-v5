pub mod config;
pub mod db;
pub mod error;
pub mod models;
pub mod openrouter;
pub mod routes;

use std::sync::Arc;

use axum::{
    Router,
    http::{Method, header},
};
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use crate::{
    config::Config,
    db::Db,
    error::AppError,
    openrouter::{DynChatClient, OpenRouterClient},
};

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub openrouter: DynChatClient,
    pub model: String,
}

pub async fn build_app(config: Config) -> Result<Router, AppError> {
    let db = Db::connect(&config.database_url).await?;
    let client = Arc::new(OpenRouterClient::new(config.openrouter_api_key.clone()));
    app_from_parts(db, client, config.openrouter_model, &config.frontend_origin)
}

pub fn app_from_parts(
    db: Db,
    openrouter: DynChatClient,
    model: String,
    frontend_origin: &str,
) -> Result<Router, AppError> {
    let state = Arc::new(AppState {
        db,
        openrouter,
        model,
    });
    let origin = frontend_origin
        .parse()
        .map_err(|source| crate::config::ConfigError::InvalidFrontendOrigin { source })?;
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::exact(origin))
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE]);

    Ok(routes::router()
        .with_state(state)
        .layer(cors)
        .layer(TraceLayer::new_for_http()))
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use async_trait::async_trait;
    use axum::{body::Body, http::Request};
    use http_body_util::BodyExt;
    use serde_json::{Value, json};
    use tower::ServiceExt;

    use super::*;
    use crate::{
        config::DEFAULT_MODEL,
        models::MessageRole,
        openrouter::{ChatClient, ChatStream, OpenRouterError, OpenRouterMessage},
    };

    #[derive(Default)]
    struct MockChatClient {
        calls: Mutex<Vec<Vec<OpenRouterMessage>>>,
    }

    #[async_trait]
    impl ChatClient for MockChatClient {
        async fn complete(
            &self,
            _model: &str,
            messages: Vec<OpenRouterMessage>,
        ) -> Result<String, OpenRouterError> {
            self.calls.lock().expect("lock calls").push(messages);
            Ok("Mock assistant response".to_owned())
        }

        async fn stream(
            &self,
            _model: &str,
            messages: Vec<OpenRouterMessage>,
        ) -> Result<ChatStream, OpenRouterError> {
            self.calls.lock().expect("lock calls").push(messages);
            Ok(Box::pin(futures_util::stream::iter([
                Ok("Mock ".to_owned()),
                Ok("stream".to_owned()),
            ])))
        }
    }

    async fn test_app() -> (Router, Arc<MockChatClient>) {
        let db = Db::in_memory().await.expect("in-memory db");
        let client = Arc::new(MockChatClient::default());
        let app = app_from_parts(
            db,
            client.clone(),
            DEFAULT_MODEL.to_owned(),
            "http://localhost:3000",
        )
        .expect("app");
        (app, client)
    }

    async fn request_json(app: Router, method: &str, uri: &str, body: Value) -> (u16, Value) {
        let response = app
            .oneshot(
                Request::builder()
                    .method(method)
                    .uri(uri)
                    .header("content-type", "application/json")
                    .body(Body::from(body.to_string()))
                    .expect("request"),
            )
            .await
            .expect("response");
        let status = response.status().as_u16();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        let value = serde_json::from_slice(&body).unwrap_or_else(|_| json!({}));
        (status, value)
    }

    async fn request_get(app: Router, uri: &str) -> (u16, Value) {
        let response = app
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        let status = response.status().as_u16();
        let body = response
            .into_body()
            .collect()
            .await
            .expect("body")
            .to_bytes();
        (status, serde_json::from_slice(&body).expect("json"))
    }

    #[tokio::test]
    async fn health_returns_ok() {
        let (app, _) = test_app().await;
        let (status, body) = request_get(app, "/health").await;

        assert_eq!(status, 200);
        assert_eq!(body["ok"], true);
        assert_eq!(body["service"], "gpt-copy-v5-backend");
    }

    #[tokio::test]
    async fn validation_errors_are_structured() {
        let (app, _) = test_app().await;
        let long_title = "x".repeat(121);
        let (status, body) = request_json(
            app,
            "POST",
            "/api/conversations",
            json!({ "title": long_title }),
        )
        .await;

        assert_eq!(status, 400);
        assert_eq!(body["error"]["code"], "validation_error");
        assert!(body["error"]["message"].as_str().unwrap().contains("title"));
    }

    #[tokio::test]
    async fn conversations_and_messages_persist() {
        let (app, _) = test_app().await;
        let (status, created) = request_json(
            app.clone(),
            "POST",
            "/api/conversations",
            json!({ "title": "Test" }),
        )
        .await;
        assert_eq!(status, 200);
        let conversation_id = created["conversation"]["id"].as_str().unwrap();

        let (status, list) = request_get(app.clone(), "/api/conversations").await;
        assert_eq!(status, 200);
        assert_eq!(list["conversations"].as_array().unwrap().len(), 1);

        let (status, messages) = request_get(
            app,
            &format!("/api/conversations/{conversation_id}/messages"),
        )
        .await;
        assert_eq!(status, 200);
        assert!(messages["messages"].as_array().unwrap().is_empty());
    }

    #[tokio::test]
    async fn send_message_uses_mock_openrouter_and_saves_both_messages() {
        let (app, client) = test_app().await;
        let (_, created) = request_json(
            app.clone(),
            "POST",
            "/api/conversations",
            json!({ "title": "Test" }),
        )
        .await;
        let conversation_id = created["conversation"]["id"].as_str().unwrap();

        let (status, sent) = request_json(
            app.clone(),
            "POST",
            &format!("/api/conversations/{conversation_id}/messages"),
            json!({ "content": "Hello model" }),
        )
        .await;

        assert_eq!(status, 200);
        assert_eq!(sent["user_message"]["role"], json!(MessageRole::User));
        assert_eq!(
            sent["assistant_message"]["content"],
            "Mock assistant response"
        );
        {
            let calls = client.calls.lock().expect("calls");
            assert_eq!(calls.len(), 1);
            assert_eq!(calls[0].last().unwrap().content, "Hello model");
        }

        let (status, messages) = request_get(
            app,
            &format!("/api/conversations/{conversation_id}/messages"),
        )
        .await;
        assert_eq!(status, 200);
        assert_eq!(messages["messages"].as_array().unwrap().len(), 2);
    }
}
