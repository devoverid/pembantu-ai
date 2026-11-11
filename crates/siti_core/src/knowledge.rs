//! Lightweight knowledge base (ingest + retrieval) for agent RAG.

use serde::{Deserialize, Serialize};
use sqlx::PgPool;

use crate::error::SitiError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeDocument {
    pub id: i64,
    pub owner_user_id: Option<i64>,
    pub conversation_id: Option<i64>,
    pub title: String,
    pub content: String,
    pub source: Option<String>,
}

pub async fn add_document(
    pool: &PgPool,
    owner_user_id: Option<i64>,
    conversation_id: Option<i64>,
    title: &str,
    content: &str,
    source: Option<&str>,
) -> Result<i64, SitiError> {
    let id: i64 = sqlx::query_scalar(
        "INSERT INTO knowledge_documents (owner_user_id, conversation_id, title, content, source)
         VALUES ($1,$2,$3,$4,$5) RETURNING id",
    )
    .bind(owner_user_id)
    .bind(conversation_id)
    .bind(title)
    .bind(content)
    .bind(source)
    .fetch_one(pool)
    .await?;
    Ok(id)
}

pub async fn list_documents(
    pool: &PgPool,
    owner_user_id: Option<i64>,
    conversation_id: Option<i64>,
    limit: i64,
) -> Result<Vec<KnowledgeDocument>, SitiError> {
    let rows = sqlx::query_as::<_, (i64, Option<i64>, Option<i64>, String, String, Option<String>)>(
        "SELECT id, owner_user_id, conversation_id, title, content, source
         FROM knowledge_documents
         WHERE ($1::bigint IS NULL OR owner_user_id = $1)
           AND ($2::bigint IS NULL OR conversation_id = $2)
         ORDER BY updated_at DESC, id DESC LIMIT $3",
    )
    .bind(owner_user_id)
    .bind(conversation_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| KnowledgeDocument {
            id: r.0,
            owner_user_id: r.1,
            conversation_id: r.2,
            title: r.3,
            content: r.4,
            source: r.5,
        })
        .collect())
}

/// Simple full-text-ish search over title/content (ILIKE fallback-friendly). Returns top matches.
pub async fn search_documents(
    pool: &PgPool,
    query: &str,
    owner_user_id: Option<i64>,
    limit: i64,
) -> Result<Vec<KnowledgeDocument>, SitiError> {
    let q = format!("%{}%", query.trim());
    let rows = sqlx::query_as::<_, (i64, Option<i64>, Option<i64>, String, String, Option<String>)>(
        "SELECT id, owner_user_id, conversation_id, title, content, source
         FROM knowledge_documents
         WHERE ($2::bigint IS NULL OR owner_user_id = $2)
           AND (title ILIKE $1 OR content ILIKE $1)
         ORDER BY updated_at DESC, id DESC LIMIT $3",
    )
    .bind(q)
    .bind(owner_user_id)
    .bind(limit)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|r| KnowledgeDocument {
            id: r.0,
            owner_user_id: r.1,
            conversation_id: r.2,
            title: r.3,
            content: r.4,
            source: r.5,
        })
        .collect())
}

pub async fn delete_document(pool: &PgPool, id: i64, owner_user_id: Option<i64>) -> Result<bool, SitiError> {
    let res = if let Some(owner) = owner_user_id {
        sqlx::query("DELETE FROM knowledge_documents WHERE id = $1 AND owner_user_id = $2")
            .bind(id)
            .bind(owner)
            .execute(pool)
            .await?
    } else {
        sqlx::query("DELETE FROM knowledge_documents WHERE id = $1")
            .bind(id)
            .execute(pool)
            .await?
    };
    Ok(res.rows_affected() > 0)
}
