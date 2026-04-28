use std::{pin::Pin, sync::Arc};

use async_trait::async_trait;
use axum::http::StatusCode;
use futures_util::{Stream, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};

pub type ChatStream = Pin<Box<dyn Stream<Item = Result<String, OpenRouterError>> + Send>>;
pub type DynChatClient = Arc<dyn ChatClient>;

#[async_trait]
pub trait ChatClient: Send + Sync {
    async fn complete(
        &self,
        model: &str,
        messages: Vec<OpenRouterMessage>,
    ) -> Result<String, OpenRouterError>;

    async fn stream(
        &self,
        model: &str,
        messages: Vec<OpenRouterMessage>,
    ) -> Result<ChatStream, OpenRouterError>;
}

#[derive(Clone)]
pub struct OpenRouterClient {
    http: Client,
    api_key: Option<String>,
    base_url: String,
}

impl OpenRouterClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            http: Client::new(),
            api_key,
            base_url: "https://openrouter.ai/api/v1".to_owned(),
        }
    }

    #[cfg(test)]
    pub fn with_base_url(api_key: Option<String>, base_url: String) -> Self {
        Self {
            http: Client::new(),
            api_key,
            base_url,
        }
    }

    fn api_key(&self) -> Result<&str, OpenRouterError> {
        self.api_key
            .as_deref()
            .ok_or(OpenRouterError::MissingApiKey)
    }
}

#[async_trait]
impl ChatClient for OpenRouterClient {
    async fn complete(
        &self,
        model: &str,
        messages: Vec<OpenRouterMessage>,
    ) -> Result<String, OpenRouterError> {
        let payload = OpenRouterChatRequest {
            model,
            messages,
            stream: false,
        };
        let response = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(self.api_key()?)
            .header("HTTP-Referer", "http://localhost:3000")
            .header("X-Title", "gpt-copy-v5")
            .json(&payload)
            .send()
            .await
            .map_err(OpenRouterError::Transport)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(OpenRouterError::BadStatus { status, body });
        }
        let body = response
            .json::<OpenRouterChatResponse>()
            .await
            .map_err(OpenRouterError::Transport)?;
        body.choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .filter(|content| !content.trim().is_empty())
            .ok_or_else(|| OpenRouterError::InvalidResponse("missing assistant content".to_owned()))
    }

    async fn stream(
        &self,
        model: &str,
        messages: Vec<OpenRouterMessage>,
    ) -> Result<ChatStream, OpenRouterError> {
        let payload = OpenRouterChatRequest {
            model,
            messages,
            stream: true,
        };
        let response = self
            .http
            .post(format!("{}/chat/completions", self.base_url))
            .bearer_auth(self.api_key()?)
            .header("HTTP-Referer", "http://localhost:3000")
            .header("X-Title", "gpt-copy-v5")
            .json(&payload)
            .send()
            .await
            .map_err(OpenRouterError::Transport)?;
        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(OpenRouterError::BadStatus { status, body });
        }

        let bytes = response.bytes_stream();
        let stream = async_stream::try_stream! {
            futures_util::pin_mut!(bytes);
            let mut buffer = String::new();

            while let Some(chunk) = bytes.next().await {
                let chunk = chunk.map_err(OpenRouterError::Transport)?;
                buffer.push_str(&String::from_utf8_lossy(&chunk));

                while let Some(index) = buffer.find("\n\n") {
                    let frame = buffer[..index].to_owned();
                    buffer = buffer[index + 2..].to_owned();
                    for line in frame.lines() {
                        let Some(data) = line.strip_prefix("data:") else {
                            continue;
                        };
                        let data = data.trim();
                        if data == "[DONE]" {
                            return;
                        }
                        let event = serde_json::from_str::<OpenRouterStreamChunk>(data)
                            .map_err(|source| OpenRouterError::InvalidResponse(source.to_string()))?;
                        if let Some(content) = event
                            .choices
                            .into_iter()
                            .next()
                            .and_then(|choice| choice.delta.content)
                            .filter(|content| !content.is_empty())
                        {
                            yield content;
                        }
                    }
                }
            }
        };

        Ok(Box::pin(stream))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OpenRouterMessage {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Serialize)]
struct OpenRouterChatRequest<'a> {
    model: &'a str,
    messages: Vec<OpenRouterMessage>,
    stream: bool,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChatResponse {
    choices: Vec<OpenRouterChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoiceMessage {
    content: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterStreamChunk {
    choices: Vec<OpenRouterStreamChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterStreamChoice {
    delta: OpenRouterStreamDelta,
}

#[derive(Debug, Deserialize)]
struct OpenRouterStreamDelta {
    content: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum OpenRouterError {
    #[error("OPENROUTER_API_KEY is not configured")]
    MissingApiKey,
    #[error("OpenRouter transport failed: {0}")]
    Transport(reqwest::Error),
    #[error("OpenRouter returned {status}: {body}")]
    BadStatus { status: StatusCode, body: String },
    #[error("OpenRouter response was invalid: {0}")]
    InvalidResponse(String),
}

impl OpenRouterError {
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::MissingApiKey => StatusCode::INTERNAL_SERVER_ERROR,
            Self::Transport(_) | Self::BadStatus { .. } | Self::InvalidResponse(_) => {
                StatusCode::BAD_GATEWAY
            }
        }
    }
}
