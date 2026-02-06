use std::collections::HashMap;
use tracing::{error, info};
use uuid::Uuid;
use loro::LoroDoc;
use crate::models::{ColabModel, ColabPackage};
use crate::db::dbcolab::{self, DocumentStreamRow};
use crate::ws::docctx::DocContext;

pub async fn fetch_doc_snapshot_from_db(org_id: &str, doc_id: &str, version: Option<u32>) -> Result<Option<(Vec<u8>, DocContext)>, String> {
        info!("Loading document: {}", doc_id);

        // Parse the doc_id as an UUID
        let doc_uuid = match Uuid::parse_str(&doc_id) {
            Ok(uuid) => uuid,
            Err(e) => {
                error!("Invalid document UUID '{}': {}", doc_id, e);
                return Err(format!("Invalid document UUID: {}", e));
            }
        };
        
        // Get database connection
        let db = match dbcolab::get_db() {
            Some(db) => db,
            None => {
                error!("Database not initialized");
                return Err("Database not initialized".to_string());
            }
        };
        
        // Load document from database
        let doc_data = match db.load_colab_doc(&org_id, doc_uuid).await {
            Ok(Some(doc)) => doc,
            Ok(None) => {
                info!("Document not found: {}", doc_uuid.to_string());
                return Ok(None);
            }
            Err(e) => {
                error!("Database error loading document '{}': {}", doc_uuid.to_string(), e);
                return Err(format!("Database error: {}", e));
            }
        };
        
        // Iterate over the streams and search for the stream with name "main" and the highest version.
        let mut main_stream: Option<&DocumentStreamRow> = None;
        let mut main_stream_bytes: Option<&Vec<u8>> = None;
        
        // If a version is specified, we look for that specific version of the main stream. If not, we look for the main stream with the highest version.
        let stream_version = match version {
            Some(v) => {
                for stream in &doc_data.streams {
                    if stream.name == "main" && stream.version == v {
                        if let Some(content) = &stream.content {
                            main_stream_bytes = Some(content);
                            main_stream = Some(stream);
                            break;
                        }
                    }
                }
                v
            },
            None => {
                let mut highest_version: u32 = 0;
                for stream in &doc_data.streams {
                    if stream.name == "main" && stream.version > highest_version {
                        if let Some(content) = &stream.content {
                            main_stream_bytes = Some(content);
                            main_stream = Some(stream);
                            highest_version = stream.version;
                        }
                    }
                }
                highest_version
            },
        };


        // Check if we found content for the highest main stream
        if main_stream_bytes.is_none() || main_stream.is_none() {
            if let Some(ref json_value) = doc_data.json {
                // We need to generate the loro doc from the json in the statement.
                
                // Parse the json as ColabModel
                let doc_model: ColabModel = match serde_json::from_value(json_value.clone()) {
                    Ok(model) => model,
                    Err(e) => {
                        error!("Failed to parse ColabModel JSON for document '{}': {}", doc_uuid.to_string(), e);
                        error!("JSON content: {}", serde_json::to_string_pretty(&json_value).unwrap_or_else(|_| "Unable to serialize".to_string()));
                        return Err(format!("Failed to parse JSON: {}", e));
                    }
                };

                // Convert ColabModel to LoroDoc
                let loro_doc: LoroDoc = match crate::models::lorodoc::colab_to_loro_doc(&doc_model) {
                    Some(doc) => doc,
                    None => {
                        error!("Failed to convert ColabModel to LoroDoc for document '{}'", doc_uuid.to_string());
                        return Err("Failed to convert ColabModel to LoroDoc".to_string());
                    }
                };

                // Export the LoroDoc as a byte stream
                let snapshot = loro_doc.export(loro::ExportMode::Snapshot).unwrap();

                // Create the peer map with the current peer
                let mut peer_map: HashMap<u64, String> = HashMap::new();
                peer_map.insert(loro_doc.peer_id(), "s/colabri-doc".to_string());

                // Put it in a ColabPackage
                // Create the ColabPackage to store in the database
                let colab_package = ColabPackage {
                    snapshot: snapshot.clone(),
                    peer_map: peer_map.clone(),
                };

                // Serialize the ColabPackage to CBOR
                let blob = match serde_cbor::to_vec(&colab_package) {
                    Ok(data) => data,
                    Err(e) => {
                        error!("Failed to serialize ColabPackage for document '{}': {}", doc_uuid.to_string(), e);
                        return Err(format!("Failed to serialize ColabPackage: {}", e));
                    }
                };

                // Store the generated snapshot as a new stream in the database
                let docstream_id = match db.insert_doc_stream(
                    &org_id,
                    doc_uuid,
                    blob
                ).await {
                    Ok(id) => id,
                    Err(e) => {
                        error!("Failed to insert document stream for document '{}': {}", doc_uuid.to_string(), e);
                        return Err(format!("Failed to insert document stream: {}", e));
                    }
                };

                // Create DocContext
                let context = DocContext {
                    org: org_id.to_string(),
                    doc_id: doc_uuid.clone(),
                    doc_stream_id: docstream_id.clone(),
                    doc_version: stream_version,
                    doc_owner: doc_data.owner.clone(),
                    peer_map: peer_map.clone(),
                    last_updating_peer: Some(loro_doc.peer_id()),
                };

                return Ok(Some((snapshot, context)));
            }
            // No stream and no json
            else {
                error!("No content found for document '{}'", doc_uuid.to_string());
                return Err("No content found".to_string());
            }
        }
        // Import the content into the LoroDoc
        else {

            // Deserialize the CBOR formatted "main_stream_bytes" into a ColabPackage
            let colab_package : ColabPackage = match serde_cbor::from_slice(&main_stream_bytes.unwrap()) {
                Ok(pkg) => pkg,
                Err(e) => {
                    error!("Failed to deserialize ColabPackage for document '{}': {}", doc_uuid.to_string(), e);
                    return Err(format!("Failed to deserialize ColabPackage: {}", e));
                }
            };

            // Get the peer map
            let loro_snapshot = colab_package.snapshot;
            let peer_map = colab_package.peer_map;

            // Create DocContext
            let context = DocContext {
                org: org_id.to_string(),
                doc_id: doc_uuid.clone(),
                doc_stream_id: main_stream.unwrap().id.clone(),
                doc_version: stream_version,
                doc_owner: doc_data.owner.clone(),
                peer_map: peer_map,
                last_updating_peer: None,
            };

            info!("Successfully loaded document: {} ({} bytes)", doc_uuid.to_string(), main_stream_bytes.unwrap().len());
            return Ok(Some((loro_snapshot, context)));
        }
}
