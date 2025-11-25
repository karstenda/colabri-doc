use std::collections::HashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColabDocumentType {
    #[serde(rename = "doc-statement")]
    TextDoc,
    #[serde(rename = "doc-sheet")]
    SheetDoc,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ColabDocPermission {
    View,
    Edit,
    Manage,
    AddRemove,
    Delete,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColabCommentType {
    User,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColabCommentState {
    Open,
    Resolved,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabComment {
    #[serde(rename = "type")]
    pub r#type: ColabCommentType,
    pub state: ColabCommentState,
    pub author: String,
    pub text: String,
    pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabStatementDoc {
    #[serde(rename = "type")]
    pub r#type: ColabDocumentType,
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub acl: HashMap<ColabDocPermission, Vec<String>>,
    pub content: HashMap<String, ColabStatement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabStatement {
    pub text: String,
    pub acl: HashMap<ColabDocPermission, Vec<String>>,
    pub comments: Vec<ColabComment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabSheetDoc {
    #[serde(rename = "type")]
    pub r#type: ColabDocumentType,
    #[serde(rename = "contentType")]
    pub content_type: String,
    pub owner: String,
    pub acl: HashMap<ColabDocPermission, Vec<String>>,
    pub content: Vec<ColabBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColabBlockType {
    Text,
    Graphic,
    TextList,
    Barcode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ColabBlock {
    #[serde(rename = "text")]
    Text(ColabDocTextItem),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabDocTextItem {
    pub text: String,
    pub acl: HashMap<ColabDocPermission, Vec<String>>,
}