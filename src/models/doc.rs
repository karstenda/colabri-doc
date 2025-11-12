
use loro::LoroDoc;
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, base64::Base64};

/// Request body for creating an item
#[serde_as]
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct SerializedColabDoc {
    pub name: String,
    pub id: String,
    #[serde_as(as = "Base64")]
    pub loro_doc: Vec<u8>,
}

pub struct ColabDoc {
    pub name: String,
    pub id: String,
    pub loro_doc: LoroDoc,
}