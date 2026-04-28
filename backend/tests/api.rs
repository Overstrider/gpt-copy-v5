use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Request, StatusCode, header},
};
use gpt_copy_v5_backend::{
    app_from_parts,
    config::DEFAULT_MODEL,
    db::Db,
    openrouter::{ChatClient, ChatStream, OpenRouterError, OpenRouterMessage},
};
use serde_json::{Value, json};
use tower::ServiceExt;

#[derive(Default)]
struct MockOpenRouter {
    calls: Mutex<Vec<Vec<OpenRouterMessage>>>,
}

#[async_trait]
impl ChatClient for MockOpenRouter {
    async fn complete(
        &self,
        _model: &str,
        messages: Vec<OpenRouterMessage>,
    ) -> Result<String, OpenRouterError> {
        self.calls.lock().expect("calls").push(messages);
        Ok("Mocked assistant reply".to_owned())
    }

    async fn stream(
        &self,
        _model: &str,
        messages: Vec<OpenRouterMessage>,
    ) -> Result<ChatStream, OpenRouterError> {
        self.calls.lock().expect("calls").push(messages);
        Ok(Box::pin(futures_util::stream::iter([
            Ok("Mocked ".to_owned()),
            Ok("assistant reply".to_owned()),
        ])))
    }
}

async fn test_app(mock: Arc<MockOpenRouter>) -> Router {
    let db = Db::in_memory().await.expect("db");
    app_from_parts(db, mock, DEFAULT_MODEL.to_owned(), "http://localhost:3000").expect("app")
}

async fn json_request(app: Router, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("body");
    (status, serde_json::from_slice(&bytes).expect("json"))
}

#[tokio::test]
async fn stream_uses_mocked_openrouter_and_persists_assistant_reply() {
    let mock = Arc::new(MockOpenRouter::default());
    let app = test_app(mock.clone()).await;

    let (_, created) = json_request(
        app.clone(),
        "POST",
        "/api/conversations",
        json!({ "title": "Chat" }),
    )
    .await;
    let conversation_id = created["conversation"]["id"].as_str().unwrap();

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/api/conversations/{conversation_id}/stream"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "content": "What is up?" }).to_string()))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), StatusCode::OK);
    assert!(
        response.headers()[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/event-stream")
    );
    let stream_body = String::from_utf8(
        to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("body")
            .to_vec(),
    )
    .unwrap();
    assert!(stream_body.contains("Mocked"));
    assert!(stream_body.contains("assistant reply"));

    {
        let captured = mock.calls.lock().expect("calls");
        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].last().unwrap().content, "What is up?");
    }

    let (_, messages) = json_request(
        app,
        "GET",
        &format!("/api/conversations/{conversation_id}/messages"),
        json!(null),
    )
    .await;
    assert_eq!(messages["messages"].as_array().unwrap().len(), 2);
    assert_eq!(messages["messages"][0]["role"], "user");
    assert_eq!(messages["messages"][1]["role"], "assistant");
    assert_eq!(messages["messages"][1]["content"], "Mocked assistant reply");
}
