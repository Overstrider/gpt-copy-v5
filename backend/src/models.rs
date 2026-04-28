use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Conversation {
    pub id: Uuid,
    pub title: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct Message {
    pub id: Uuid,
    pub conversation_id: Uuid,
    pub role: MessageRole,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    System,
    User,
    Assistant,
}

impl MessageRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        }
    }

    pub fn from_db(value: &str) -> Result<Self, AppError> {
        match value {
            "system" => Ok(Self::System),
            "user" => Ok(Self::User),
            "assistant" => Ok(Self::Assistant),
            _ => Err(AppError::validation("message role is invalid")),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub ok: bool,
    pub service: &'static str,
    pub version: &'static str,
}

#[derive(Debug, Deserialize)]
pub struct CreateConversationRequest {
    pub title: Option<String>,
}

impl CreateConversationRequest {
    pub fn validated_title(&self) -> Result<String, AppError> {
        match self
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            Some(title) if title.chars().count() <= 120 => Ok(title.to_owned()),
            Some(_) => Err(AppError::validation(
                "title must be 120 characters or fewer",
            )),
            None => Ok("New chat".to_owned()),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content: String,
}

impl SendMessageRequest {
    pub fn validated_content(&self) -> Result<String, AppError> {
        let content = self.content.trim();
        if content.is_empty() {
            return Err(AppError::validation("content is required"));
        }
        if content.chars().count() > 16_000 {
            return Err(AppError::validation(
                "content must be 16000 characters or fewer",
            ));
        }
        Ok(content.to_owned())
    }
}

#[derive(Debug, Serialize)]
pub struct ConversationListResponse {
    pub conversations: Vec<Conversation>,
}

#[derive(Debug, Serialize)]
pub struct ConversationResponse {
    pub conversation: Conversation,
}

#[derive(Debug, Serialize)]
pub struct MessagesResponse {
    pub messages: Vec<Message>,
}

#[derive(Debug, Serialize)]
pub struct SendMessageResponse {
    pub user_message: Message,
    pub assistant_message: Message,
}
