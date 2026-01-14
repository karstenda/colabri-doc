use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use sqlx::types::Json;
use sqlx::{Error as SqlxError, Row};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::OnceCell;
use tracing::{error, info};

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

/// Document Row from database
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ViewableDocumentRow {
    pub id: uuid::Uuid,
    pub name: String,
    #[sqlx(rename = "type")]
    pub doc_type: String,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
    pub deleted: bool,
    pub org: String,
}

/// Document with full metadata from the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabDocument {
    pub id: uuid::Uuid,
    pub name: String,
    pub doc_type: String,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
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
    #[serde(deserialize_with = "deserialize_base64_content")]
    pub content: Option<Vec<u8>>,
    pub pointer: Option<String>,
    pub size: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub created_by: String,
    pub updated_by: String,
    pub deleted: bool,
}

fn deserialize_base64_content<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use base64::{engine::general_purpose, Engine as _};

    let opt: Option<String> = Option::deserialize(deserializer)?;
    match opt {
        Some(s) => general_purpose::STANDARD
            .decode(s)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
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
    pub created_by: String,
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
            .max_connections(20) // Increased from 5 to support more concurrent operations
            .min_connections(2) // Keep some connections alive
            .acquire_timeout(Duration::from_secs(30)) // Increased from 3s to 30s
            .idle_timeout(Duration::from_secs(600)) // Close idle connections after 10 minutes
            .max_lifetime(Duration::from_secs(1800)) // Recycle connections after 30 minutes
            .connect(database_url)
            .await?;

        info!("Database connection pool created successfully");

        Ok(Self { pool })
    }

    /// Get a reference to the connection pool
    pub fn _pool(&self) -> &PgPool {
        &self.pool
    }

    /// Get a document if the user has view access to it
    ///
    /// # Arguments
    /// * `org` - Organization identifier
    /// * `document_id` - The ID of the document to check
    /// * `principals` - List of principals (user ID, roles, etc.)
    ///
    /// # Returns
    /// * `Result<Option<ViewableDocumentRow>, SqlxError>` - The document if found and accessible
    pub async fn get_viewable_document(
        &self,
        org: &str,
        document_id: uuid::Uuid,
        principals: &[String],
    ) -> Result<Option<ViewableDocumentRow>, SqlxError> {
        // Log pool stats before acquiring connection
        let pool_idle = self.pool.num_idle() as u32;
        let pool_size = self.pool.size();
        info!(
            "Checking view access for doc {} in org {}. Pool connections: {} idle, {} in use",
            document_id,
            org,
            pool_idle,
            pool_size.saturating_sub(pool_idle)
        );

        // Begin a transaction
        let mut tx = match self.pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                error!(
                    "Failed to acquire connection from pool: {}. Pool state: {} idle, {} total",
                    e,
                    self.pool.num_idle(),
                    self.pool.size()
                );
                return Err(e);
            }
        };

        // Set the policy context
        let safe_org = org.replace("'", "''");
        let policy_sql = format!("SET LOCAL app.orgs = '{}'", safe_org);

        sqlx::query(&policy_sql).execute(&mut *tx).await?;

        let query_sql = r#"
            SELECT DISTINCT d.*
            FROM documents d
            LEFT JOIN document_acl da ON d.id = da.document
            WHERE
                d.org = $1
                AND (
                        (da.permission = 'view' AND da.prpl = ANY($2::text[])) OR
                        d.owner = ANY($2::text[]) OR
                        CONCAT($1, '/f/admin') = ANY($2::text[]) OR
                        'r/Colabri-CloudAdmin' = ANY($2::text[])
                )
                AND d.id = $3
                AND d.deleted = FALSE
        "#;

        let document = sqlx::query_as::<_, ViewableDocumentRow>(query_sql)
            .bind(org)
            .bind(principals)
            .bind(document_id)
            .fetch_optional(&mut *tx)
            .await?;

        tx.commit().await?;

        Ok(document)
    }

    /// Load a colab document by ID with ACL authorization
    ///
    /// # Arguments
    /// * `id` - Document UUID
    /// * `org` - Organization identifier
    ///
    /// # Returns
    /// * `Result<Option<ColabDocument>, SqlxError>` - Document with metadata or None if not found/unauthorized
    pub async fn load_colab_doc(
        &self,
        org: &str,
        document_id: uuid::Uuid,
    ) -> Result<Option<ColabDocument>, SqlxError> {
        // Log pool stats before acquiring connection
        let pool_idle = self.pool.num_idle() as u32;
        let pool_size = self.pool.size();
        info!(
            "Loading document {} for org {}. Pool connections: {} idle, {} in use",
            document_id,
            org,
            pool_idle,
            pool_size.saturating_sub(pool_idle)
        );

        // Begin a transaction
        let mut tx = match self.pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to acquire connection from pool for document {}: {}. Pool state: {} idle, {} total", 
                       document_id, e, self.pool.num_idle(), self.pool.size());
                return Err(e);
            }
        };

        // Set the policy context
        // Note: SET LOCAL doesn't support bind parameters, so we must escape single quotes
        let safe_org = org.replace("'", "''");
        let policy_sql = format!("SET LOCAL app.orgs = '{}'", safe_org);

        sqlx::query(&policy_sql).execute(&mut *tx).await?;

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
                CASE d.type
                    WHEN 'colab-statement' THEN st.json
                    WHEN 'colab-sheet' THEN sh.json
                END AS colab_json,
                CASE d.type
                    WHEN 'colab-statement' THEN st.synced
                    WHEN 'colab-sheet' THEN sh.synced
                END AS colab_synced,
                COALESCE(
                    (SELECT json_agg(da.*) FROM document_acl da WHERE da.document = d.id),
                    '[]'
                ) AS acls,
                COALESCE(
                    (SELECT json_agg(
                        json_build_object(
                            'org', ds.org,
                            'id', ds.id,
                            'name', ds.name,
                            'document', ds.document,
                            'version', ds.version,
                            'content', replace(encode(ds.content, 'base64'), E'\n', ''),
                            'pointer', ds.pointer,
                            'size', ds.size,
                            'created_at', ds.created_at,
                            'updated_at', ds.updated_at,
                            'created_by', ds.created_by,
                            'updated_by', ds.updated_by,
                            'deleted', ds.deleted
                        )
                    ) FROM document_streams ds WHERE ds.document = d.id AND ds.deleted = FALSE),
                    '[]'
                ) AS streams
            FROM documents d
                LEFT JOIN document_statements st ON d.id = st.document
                LEFT JOIN document_sheets sh ON d.id = sh.document
            WHERE 
                d.org = $1 
                AND d.id = $2 
                AND d.deleted = FALSE;
        "#;

        let row = sqlx::query(query_sql)
            .bind(org)
            .bind(document_id)
            .fetch_optional(&mut *tx)
            .await?;

        // Commit the transaction
        tx.commit().await?;

        match row {
            Some(row) => {
                let name: String = row.try_get("name")?;
                info!("Document '{}' loaded successfully for org '{}'", name, org);

                // Let's deserialize the streams and acls
                let streams: Vec<DocumentStreamRow> =
                    serde_json::from_value(row.try_get("streams")?)
                        .map_err(|e| SqlxError::Decode(Box::new(e)))?;
                let acls: Vec<DocumentAclRow> = serde_json::from_value(row.try_get("acls")?)
                    .map_err(|e| SqlxError::Decode(Box::new(e)))?;

                // Use sqlx::types::Json wrapper for JSONB column
                let json_wrapped: Option<Json<serde_json::Value>> = row.try_get("colab_json")?;
                let json = json_wrapped.map(|j| j.0);

                // Create the ColabDocument
                let doc = ColabDocument {
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
                    streams,
                };
                Ok(Some(doc))
            }
            None => Ok(None),
        }
    }

    /// Insert a statement document
    ///
    /// # Arguments
    /// * `org` - ID of the organization
    /// * `document_id` - Document UUID (optional, will generate if None)
    /// * `snapshot` - The snapshot of the LoroDoc to save
    ///
    /// # Returns
    /// * `Result<uuid::Uuid, SqlxError>` - Document ID
    pub async fn insert_doc_stream(
        &self,
        org: &str,
        document_id: uuid::Uuid,
        snapshot: Vec<u8>,
    ) -> Result<uuid::Uuid, SqlxError> {
        // Calculate the size of the snapshot
        let snapshot_size = snapshot.len() as i64;

        // Log pool stats before acquiring connection
        let pool_idle = self.pool.num_idle() as u32;
        let pool_size = self.pool.size();
        info!(
            "Creating document {} for org {}. Pool connections: {} idle, {} in use",
            document_id,
            org,
            pool_idle,
            pool_size.saturating_sub(pool_idle)
        );

        // Begin a transaction
        let mut tx = match self.pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                error!("Failed to acquire connection from pool for document {}: {}. Pool state: {} idle, {} total", 
                       document_id, e, self.pool.num_idle(), self.pool.size());
                return Err(e);
            }
        };

        // Set the policy context
        // Note: SET LOCAL doesn't support bind parameters, so we must escape single quotes
        let safe_org = org.replace("'", "''");
        let policy_sql = format!("SET LOCAL app.orgs = '{}'", safe_org);

        sqlx::query(&policy_sql).execute(&mut *tx).await?;

        // Execute the main query
        let query_sql = r#"
            INSERT INTO document_streams(org, document, name, content, version, size, created_by, updated_by)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            RETURNING id;
        "#;
        let row = sqlx::query(query_sql)
            .bind(org)
            .bind(document_id)
            .bind("main")
            .bind(snapshot)
            .bind(1) // version
            .bind(snapshot_size) // size
            .bind("s/colabri-doc") // created_by
            .bind("s/colabri-doc") // updated_by
            .fetch_optional(&mut *tx)
            .await?;

        // Commit the transaction
        tx.commit().await?;

        let returned_id: uuid::Uuid = row.unwrap().try_get("id")?;
        info!("Document Stream saved: {}", returned_id);
        Ok(returned_id)
    }

    /// Update a colab document
    ///
    /// # Arguments
    /// * `org` - ID of the organization
    /// * `doc_stream_id` - The UUID of the document stream to update with the new snapshot
    /// * `snapshot` - The snapshot of the LoroDoc to update
    /// * `doc_stmt_id` - The UUID of the document statement to update with the new JSON
    /// * `json` - The JSON representation of the loro document
    ///
    /// # Returns
    /// * `Result<uuid::Uuid, SqlxError>` - Stream ID
    pub async fn update_colab_doc(
        &self,
        org: &str,
        doc_id: uuid::Uuid,
        doc_type: &str,
        doc_stream_id: uuid::Uuid,
        colab_package_blob: Vec<u8>,
        json: serde_json::Value,
        by_prpl: &str,
    ) -> Result<uuid::Uuid, SqlxError> {
        // Calculate the size of the snapshot
        let content_size = colab_package_blob.len() as i64;

        // Log pool stats before acquiring connection
        let pool_idle = self.pool.num_idle() as u32;
        let pool_size = self.pool.size();
        info!(
            "Updating doc {} with stream {} for org {}. Pool connections: {} idle, {} in use",
            doc_id,
            doc_stream_id,
            org,
            pool_idle,
            pool_size.saturating_sub(pool_idle)
        );

        // Begin a transaction
        let mut tx = match self.pool.begin().await {
            Ok(tx) => tx,
            Err(e) => {
                error!(
                    "Failed to acquire connection from pool. Pool state: {} idle, {} total",
                    self.pool.num_idle(),
                    self.pool.size()
                );
                return Err(e);
            }
        };

        // Set the policy context
        // Note: SET LOCAL doesn't support bind parameters, so we must escape single quotes
        let safe_org = org.replace("'", "''");
        let policy_sql = format!("SET LOCAL app.orgs = '{}'", safe_org);

        sqlx::query(&policy_sql).execute(&mut *tx).await?;

        // Execute the main query
        let update_stream_query_sql = r#"
            UPDATE document_streams
            SET content = $1,
                size = $2,
                updated_at = NOW(),
                updated_by = $3
            WHERE org = $4
                AND id = $5
                AND deleted = FALSE
            RETURNING id;
        "#;
        let doc_stream_row = sqlx::query(update_stream_query_sql)
            .bind(colab_package_blob)
            .bind(content_size) // size
            .bind(by_prpl)
            .bind(org)
            .bind(doc_stream_id)
            .fetch_optional(&mut *tx)
            .await?;

        // Execute the document type specific update
        let doc_table_name = match doc_type {
            "colab-statement" => "document_statements",
            "colab-sheet" => "document_sheets",
            _ => {
                error!("Unsupported document type for update: {}", doc_type);
                return Err(SqlxError::RowNotFound);
            }
        };


        let update_model_query_sql = format!(r#"
        UPDATE {}
            SET json = $1,
                synced = FALSE,
                updated_at = NOW(),
                updated_by = $2
            WHERE org = $3
                AND document = $4
            RETURNING document;
        "#, doc_table_name);
        let doc_model_row = sqlx::query(&update_model_query_sql)
            .bind(json)
            .bind(by_prpl)
            .bind(org)
            .bind(doc_id)
            .fetch_optional(&mut *tx)
            .await?;

        // Commit the transaction
        tx.commit().await?;

        match (doc_stream_row, doc_model_row) {
            (Some(stream_row), Some(_model_row)) => {
                let returned_id: uuid::Uuid = stream_row.try_get("id")?;
                info!("Document Stream updated: {}", returned_id);
                Ok(returned_id)
            }
            (None, _) => {
                error!(
                    "Document stream not found for update: org={}, doc_stream={}",
                    org, doc_stream_id
                );
                Err(SqlxError::RowNotFound)
            }
            (_, None) => {
                error!(
                    "Document model not found for update: org={}, doc={}",
                    org, doc_id
                );
                Err(SqlxError::RowNotFound)
            }
        }
    }
}
