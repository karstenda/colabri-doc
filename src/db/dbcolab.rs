use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::{Error as SqlxError, Row};
use sqlx::types::Json;
use std::time::Duration;
use std::sync::Arc;
use tracing::{info, error};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tokio::sync::OnceCell;

use crate::models::colab::{ColabStatementModel};

// Global database instance
static DB: OnceCell<Arc<DbColab>> = OnceCell::const_new();

/// Initialize the global database connection
///
/// # Arguments
/// * `database_url` - PostgreSQL connection string
///
/// # Returns
/// * `Result<(), Box<dyn std::error::Error>>` - Success or error
pub async fn init_db(database_url: &str) -> Result<(), Box<dyn std::error::Error>> {
    let db = DbColab::new(database_url).await?;
    DB.set(Arc::new(db))
        .map_err(|_| "Database already initialized")?;
    Ok(())
}

/// Get the global database instance
///
/// # Returns
/// * `Option<Arc<DbColab>>` - Database instance if initialized
pub fn get_db() -> Option<Arc<DbColab>> {
    DB.get().cloned()
}

/// Document with full metadata from the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatementDocument {
    pub id: uuid::Uuid,
    pub name: String,
    pub doc_type: String,
    pub owner: uuid::Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: uuid::Uuid,
    pub updated_by: uuid::Uuid,
    pub json: Option<serde_json::Value>,
    pub acls: Vec<DocumentAclRow>,
    pub streams: Vec<DocumentStreamRow>,
}

/// Document Stream Row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentStreamRow {
    pub org: String,
    pub id: uuid::Uuid,
    pub name: String,
    pub document: uuid::Uuid,
    pub version: i32,
    pub content: Option<Vec<u8>>,
    pub pointer: Option<String>,
    pub size: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: Option<uuid::Uuid>,
    pub updated_by: Option<uuid::Uuid>,
    pub deleted: bool
}

/// Document ACL Row
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentAclRow {
    pub org: String,
    pub id: uuid::Uuid,
    pub document: uuid::Uuid,
    pub prpl: String,
    pub permission: String,
    pub created_at: DateTime<Utc>,
    pub created_by: Option<uuid::Uuid>
}

/// Database connection pool
pub struct DbColab {
    pool: PgPool,
}

impl DbColab {

    /// Create a new database connection pool
    ///
    /// # Arguments
    /// * `database_url` - PostgreSQL connection string
    ///
    /// # Returns
    /// * `Result<Self, SqlxError>` - Database connection pool or error
    pub async fn new(database_url: &str) -> Result<Self, SqlxError> {
        info!("Connecting to database...");

        let pool = PgPoolOptions::new()
            .max_connections(20)  // Increased from 5 to support more concurrent operations
            .min_connections(2)   // Keep some connections alive
            .acquire_timeout(Duration::from_secs(30))  // Increased from 3s to 30s
            .idle_timeout(Duration::from_secs(600))    // Close idle connections after 10 minutes
            .max_lifetime(Duration::from_secs(1800))   // Recycle connections after 30 minutes
            .connect(database_url)
            .await?;

        info!("Database connection pool created successfully");

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool
    pub fn _pool(&self) -> &PgPool {
        &self.pool
    }


    /// Load a statement document by ID with ACL authorization
    ///
    /// # Arguments
    /// * `id` - Document UUID
    /// * `org` - Organization identifier
    ///
    /// # Returns
    /// * `Result<Option<FullStatementDocument>, SqlxError>` - Document with metadata or None if not found/unauthorized
    pub async fn load_statement_doc(
        &self,
        org: &str,
        id: uuid::Uuid,
    ) -> Result<Option<StatementDocument>, SqlxError> {

        // Log pool stats before acquiring connection
        let pool_idle = self.pool.num_idle() as u32;
        let pool_size = self.pool.size();
        info!("Loading document {} for org {}. Pool connections: {} idle, {} in use", 
              id, org, pool_idle, pool_size.saturating_sub(pool_idle));

        // Begin a transaction
        let mut tx = match self.pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to acquire connection from pool for document {}: {}. Pool state: {} idle, {} total", 
                       id, e, self.pool.num_idle(), self.pool.size());
                return Err(e);
            }
        };

        // Set the policy context
        // Note: SET LOCAL doesn't support bind parameters, so we must escape single quotes
        let safe_org = org.replace("'", "''");
        let policy_sql = format!("SET LOCAL app.orgs = '{}'", safe_org);

        sqlx::query(&policy_sql)
            .execute(&mut *tx)
            .await?;

        // Execute the main query
        let query_sql = r#"
            SELECT
                d.id,
                d.name,
                d.type,
                d.owner,
                d.created_at,
                d.updated_at,
                d.created_by,
                d.updated_by,
                (SELECT st.json FROM document_statements st WHERE st.document = d.id LIMIT 1) as json,
                COALESCE(
                    (SELECT json_agg(da.*) FROM document_acl da WHERE da.document = d.id),
                    '[]'
                ) AS acls,
                COALESCE(
                    (SELECT json_agg(ds.*) FROM document_streams ds WHERE ds.document = d.id AND ds.deleted = FALSE),
                    '[]'
                ) AS streams
            FROM documents d 
            WHERE 
                d.org = $1 
                AND d.id = $2 
                AND d.type = 'colab-statement'
                AND d.deleted = FALSE;
        "#;

        let row = sqlx::query(query_sql)
            .bind(org)
            .bind(id)
            .fetch_optional(&mut *tx)
            .await?;

        // Commit the transaction
        tx.commit().await?;

        match row {
            Some(row) => {

                let name: String = row.try_get("name")?;
                info!("Document '{}' loaded successfully for org '{}'", name, org);

                // Let's deserialize the streams and acls
                let streams: Vec<DocumentStreamRow> = serde_json::from_value(row.try_get("streams")?)
                    .map_err(|e| SqlxError::Decode(Box::new(e)))?;
                let acls: Vec<DocumentAclRow> = serde_json::from_value(row.try_get("acls")?)
                    .map_err(|e| SqlxError::Decode(Box::new(e)))?;

                // Use sqlx::types::Json wrapper for JSONB column
                let json_wrapped: Option<Json<serde_json::Value>> = row.try_get("json")?;
                let json = json_wrapped.map(|j| j.0);
                info!("Loaded json field: {:?}", json.as_ref().map(|j| j.to_string()));

                let doc = StatementDocument {
                    id: row.try_get("id")?,
                    name: row.try_get("name")?,
                    doc_type: row.try_get("type")?,
                    owner: row.try_get("owner")?,
                    created_at: row.try_get("created_at")?,
                    updated_at: row.try_get("updated_at")?,
                    created_by: row.try_get("created_by")?,
                    updated_by: row.try_get("updated_by")?,
                    json,
                    acls,
                    streams
                };
                Ok(Some(doc))
            }
            None => Ok(None),
        }
    }

    /// Insert or update a statement document
    ///
    /// # Arguments
    /// * `id` - Document UUID (optional, will generate if None)
    /// * `doc` - Statement document to save
    ///
    /// # Returns
    /// * `Result<uuid::Uuid, SqlxError>` - Document ID
    pub async fn _upsert_statement_doc(
        &self,
        id: Option<uuid::Uuid>,
        doc: &ColabStatementModel,
    ) -> Result<uuid::Uuid, SqlxError> {
        let doc_id = id.unwrap_or_else(uuid::Uuid::new_v4);
        let content = serde_json::to_value(doc)
            .map_err(|e| SqlxError::Encode(Box::new(e)))?;

        let sql = r#"
            INSERT INTO documents (id, doc_type, content, created_at, updated_at)
            VALUES ($1, $2, $3, NOW(), NOW())
            ON CONFLICT (id) 
            DO UPDATE SET 
                content = EXCLUDED.content,
                updated_at = NOW()
            RETURNING id
        "#;

        let row = sqlx::query(sql)
            .bind(doc_id)
            .bind("doc-statement")
            .bind(content)
            .fetch_one(&self.pool)
            .await?;

        let returned_id: uuid::Uuid = row.try_get("id")?;
        info!("Statement document saved: {}", returned_id);
        Ok(returned_id)
    }
}
