use std::{str::FromStr, time::Duration};

use chrono::{DateTime, Utc};
use sqlx::{
    Row, SqlitePool,
    sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions},
};
use uuid::Uuid;

use crate::{
    error::AppError,
    models::{Conversation, Message, MessageRole},
};

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn connect(database_url: &str) -> Result<Self, AppError> {
        let options = SqliteConnectOptions::from_str(database_url)?
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(Duration::from_secs(5))
            .connect_with(options)
            .await?;
        let db = Self { pool };
        db.init().await?;
        Ok(db)
    }

    pub async fn in_memory() -> Result<Self, AppError> {
        let options = SqliteConnectOptions::from_str("sqlite::memory:")?.foreign_keys(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;
        let db = Self { pool };
        db.init().await?;
        Ok(db)
    }

    async fn init(&self) -> Result<(), AppError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL CHECK(role IN ('system', 'user', 'assistant')),
                content TEXT NOT NULL,
                created_at TEXT NOT NULL,
                FOREIGN KEY(conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            );
            "#,
        )
        .execute(&self.pool)
        .await?;

        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_messages_conversation_created ON messages(conversation_id, created_at);",
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_conversations(&self) -> Result<Vec<Conversation>, AppError> {
        let rows = sqlx::query(
            r#"
            SELECT id, title, created_at, updated_at
            FROM conversations
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(conversation_from_row).collect()
    }

    pub async fn create_conversation(&self, title: &str) -> Result<Conversation, AppError> {
        let conversation = Conversation {
            id: Uuid::new_v4(),
            title: title.to_owned(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        sqlx::query(
            r#"
            INSERT INTO conversations (id, title, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            "#,
        )
        .bind(conversation.id.to_string())
        .bind(&conversation.title)
        .bind(conversation.created_at.to_rfc3339())
        .bind(conversation.updated_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        Ok(conversation)
    }

    pub async fn get_conversation(&self, id: Uuid) -> Result<Conversation, AppError> {
        let row = sqlx::query(
            r#"
            SELECT id, title, created_at, updated_at
            FROM conversations
            WHERE id = ?1
            "#,
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await?;

        row.map(conversation_from_row)
            .transpose()?
            .ok_or_else(|| AppError::not_found("conversation"))
    }

    pub async fn list_messages(&self, conversation_id: Uuid) -> Result<Vec<Message>, AppError> {
        self.get_conversation(conversation_id).await?;
        let rows = sqlx::query(
            r#"
            SELECT id, conversation_id, role, content, created_at
            FROM messages
            WHERE conversation_id = ?1
            ORDER BY created_at ASC
            "#,
        )
        .bind(conversation_id.to_string())
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(message_from_row).collect()
    }

    pub async fn insert_message(
        &self,
        conversation_id: Uuid,
        role: MessageRole,
        content: &str,
    ) -> Result<Message, AppError> {
        self.get_conversation(conversation_id).await?;
        let message = Message {
            id: Uuid::new_v4(),
            conversation_id,
            role,
            content: content.to_owned(),
            created_at: Utc::now(),
        };

        sqlx::query(
            r#"
            INSERT INTO messages (id, conversation_id, role, content, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            "#,
        )
        .bind(message.id.to_string())
        .bind(message.conversation_id.to_string())
        .bind(message.role.as_str())
        .bind(&message.content)
        .bind(message.created_at.to_rfc3339())
        .execute(&self.pool)
        .await?;

        self.touch_conversation(conversation_id).await?;
        Ok(message)
    }

    pub async fn title_from_first_message(
        &self,
        conversation_id: Uuid,
        content: &str,
    ) -> Result<(), AppError> {
        let current = self.get_conversation(conversation_id).await?;
        if current.title != "New chat" {
            return Ok(());
        }
        let title = content
            .split_whitespace()
            .take(8)
            .collect::<Vec<_>>()
            .join(" ");
        let title = if title.is_empty() {
            "New chat".to_owned()
        } else if title.chars().count() > 80 {
            title.chars().take(80).collect()
        } else {
            title
        };
        sqlx::query("UPDATE conversations SET title = ?1, updated_at = ?2 WHERE id = ?3")
            .bind(title)
            .bind(Utc::now().to_rfc3339())
            .bind(conversation_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    async fn touch_conversation(&self, conversation_id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE conversations SET updated_at = ?1 WHERE id = ?2")
            .bind(Utc::now().to_rfc3339())
            .bind(conversation_id.to_string())
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

fn conversation_from_row(row: sqlx::sqlite::SqliteRow) -> Result<Conversation, AppError> {
    Ok(Conversation {
        id: parse_uuid(row.get::<String, _>("id"))?,
        title: row.get("title"),
        created_at: parse_datetime(row.get::<String, _>("created_at"))?,
        updated_at: parse_datetime(row.get::<String, _>("updated_at"))?,
    })
}

fn message_from_row(row: sqlx::sqlite::SqliteRow) -> Result<Message, AppError> {
    Ok(Message {
        id: parse_uuid(row.get::<String, _>("id"))?,
        conversation_id: parse_uuid(row.get::<String, _>("conversation_id"))?,
        role: MessageRole::from_db(&row.get::<String, _>("role"))?,
        content: row.get("content"),
        created_at: parse_datetime(row.get::<String, _>("created_at"))?,
    })
}

fn parse_uuid(value: String) -> Result<Uuid, AppError> {
    Uuid::parse_str(&value).map_err(|_| AppError::validation("stored uuid is invalid"))
}

fn parse_datetime(value: String) -> Result<DateTime<Utc>, AppError> {
    DateTime::parse_from_rfc3339(&value)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|_| AppError::validation("stored timestamp is invalid"))
}
