use std::{convert::Infallible, sync::Arc};

use axum::{
    Json, Router,
    extract::{Path, State, rejection::JsonRejection},
    response::sse::{Event, Sse},
    routing::{get, post},
};
use futures_util::StreamExt;
use uuid::Uuid;

use crate::{
    AppState,
    error::AppError,
    models::{
        ConversationListResponse, ConversationResponse, CreateConversationRequest, HealthResponse,
        MessageRole, MessagesResponse, SendMessageRequest, SendMessageResponse,
    },
    openrouter::OpenRouterMessage,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/health", get(health))
        .route(
            "/api/conversations",
            get(list_conversations).post(create_conversation),
        )
        .route(
            "/api/conversations/{conversation_id}/messages",
            get(list_messages).post(send_message),
        )
        .route(
            "/api/conversations/{conversation_id}/stream",
            post(stream_message),
        )
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "gpt-copy-v5-backend",
        version: env!("CARGO_PKG_VERSION"),
    })
}

async fn list_conversations(
    State(state): State<Arc<AppState>>,
) -> Result<Json<ConversationListResponse>, AppError> {
    Ok(Json(ConversationListResponse {
        conversations: state.db.list_conversations().await?,
    }))
}

async fn create_conversation(
    State(state): State<Arc<AppState>>,
    payload: Result<Json<CreateConversationRequest>, JsonRejection>,
) -> Result<Json<ConversationResponse>, AppError> {
    let Json(payload) = parse_json(payload)?;
    let title = payload.validated_title()?;
    let conversation = state.db.create_conversation(&title).await?;
    Ok(Json(ConversationResponse { conversation }))
}

async fn list_messages(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Result<Json<MessagesResponse>, AppError> {
    let conversation_id = parse_uuid(&conversation_id)?;
    Ok(Json(MessagesResponse {
        messages: state.db.list_messages(conversation_id).await?,
    }))
}

async fn send_message(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    payload: Result<Json<SendMessageRequest>, JsonRejection>,
) -> Result<Json<SendMessageResponse>, AppError> {
    let conversation_id = parse_uuid(&conversation_id)?;
    let Json(payload) = parse_json(payload)?;
    let content = payload.validated_content()?;
    let mut history = state.db.list_messages(conversation_id).await?;
    let user_message = state
        .db
        .insert_message(conversation_id, MessageRole::User, &content)
        .await?;
    state
        .db
        .title_from_first_message(conversation_id, &content)
        .await?;
    history.push(user_message.clone());

    let assistant_content = state
        .openrouter
        .complete(&state.model, to_openrouter_messages(history))
        .await?;
    let assistant_message = state
        .db
        .insert_message(
            conversation_id,
            MessageRole::Assistant,
            assistant_content.trim(),
        )
        .await?;

    Ok(Json(SendMessageResponse {
        user_message,
        assistant_message,
    }))
}

async fn stream_message(
    State(state): State<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    payload: Result<Json<SendMessageRequest>, JsonRejection>,
) -> Result<Sse<impl futures_util::Stream<Item = Result<Event, Infallible>>>, AppError> {
    let conversation_id = parse_uuid(&conversation_id)?;
    let Json(payload) = parse_json(payload)?;
    let content = payload.validated_content()?;
    let mut history = state.db.list_messages(conversation_id).await?;
    let user_message = state
        .db
        .insert_message(conversation_id, MessageRole::User, &content)
        .await?;
    state
        .db
        .title_from_first_message(conversation_id, &content)
        .await?;
    history.push(user_message);

    let provider_stream = state
        .openrouter
        .stream(&state.model, to_openrouter_messages(history))
        .await?;
    let db = state.db.clone();

    let stream = async_stream::stream! {
        futures_util::pin_mut!(provider_stream);
        let mut assistant_content = String::new();

        while let Some(item) = provider_stream.next().await {
            match item {
                Ok(chunk) => {
                    assistant_content.push_str(&chunk);
                    yield Ok(Event::default().event("chunk").data(chunk));
                }
                Err(error) => {
                    yield Ok(Event::default().event("error").data(error.to_string()));
                    return;
                }
            }
        }

        if !assistant_content.trim().is_empty()
            && let Err(error) = db
                .insert_message(
                    conversation_id,
                    MessageRole::Assistant,
                    assistant_content.trim(),
                )
                .await
        {
            yield Ok(Event::default().event("error").data(error.to_string()));
            return;
        }

        yield Ok(Event::default().event("done").data("{}"));
    };

    Ok(Sse::new(stream))
}

fn parse_json<T>(payload: Result<Json<T>, JsonRejection>) -> Result<Json<T>, AppError> {
    payload.map_err(|err| AppError::validation(format!("invalid JSON body: {err}")))
}

fn parse_uuid(value: &str) -> Result<Uuid, AppError> {
    Uuid::parse_str(value).map_err(|_| AppError::validation("conversation_id must be a UUID"))
}

fn to_openrouter_messages(messages: Vec<crate::models::Message>) -> Vec<OpenRouterMessage> {
    messages
        .into_iter()
        .map(|message| OpenRouterMessage {
            role: message.role.as_str().to_owned(),
            content: message.content,
        })
        .collect()
}
