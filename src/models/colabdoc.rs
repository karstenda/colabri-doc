use chrono::{DateTime, Utc};
use serde::{de, Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::option::Option;

#[derive(Serialize, Deserialize)]
pub struct ColabPackage {
    pub snapshot: Vec<u8>,
    pub peer_map: HashMap<u64, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ColabModelType {
    #[serde(rename = "colab-statement")]
    ColabStatement,
    #[serde(rename = "colab-sheet")]
    ColabSheet,
}

impl fmt::Display for ColabModelType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColabModelType::ColabStatement => write!(f, "colab-statement"),
            ColabModelType::ColabSheet => write!(f, "colab-sheet"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum ColabModelPermission {
    View,
    Edit,
    Manage,
    #[serde(rename = "add-remove")]
    AddRemove,
    Delete,
}

impl fmt::Display for ColabModelPermission {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColabModelPermission::View => write!(f, "view"),
            ColabModelPermission::Edit => write!(f, "edit"),
            ColabModelPermission::Manage => write!(f, "manage"),
            ColabModelPermission::AddRemove => write!(f, "add-remove"),
            ColabModelPermission::Delete => write!(f, "delete"),
        }
    }
}

#[derive(Debug)]
pub enum ColabModel {
    Statement(ColabStatementModel),
    Sheet(ColabSheetModel),
}


#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabModelProperties {
    #[serde(rename = "type")]
    pub r#type: ColabModelType,
    #[serde(rename = "contentType")]
    pub content_type: String,
    #[serde(
        rename = "countryCodes",
        skip_serializing_if = "Option::is_none"
    )]
    pub country_codes: Option<Vec<String>>,
    #[serde(
        rename = "langCodes",
        skip_serializing_if = "Option::is_none"
    )]
    pub lang_codes: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabSheetModel {
    pub properties: ColabModelProperties,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub approvals: HashMap<String, ColabApproval>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub acls: HashMap<ColabModelPermission, Vec<String>>,
    pub content: Vec<ColabSheetBlock>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ColabSheetBlock {
    #[serde(rename = "text")]
    Text(ColabSheetTextBlock),
    #[serde(rename = "statement-grid")]
    StatementGrid(ColabSheetStatementGridBlock),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabSheetTextBlock {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub acls: HashMap<ColabModelPermission, Vec<String>>,
    #[serde(rename = "textElement")]
    pub text_element: TextElement,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub approvals: HashMap<String, ColabApproval>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabSheetStatementGridBlock {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub acls: HashMap<ColabModelPermission, Vec<String>>,
    pub rows: Vec<ColabSheetStatementGridRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabSheetStatementGridRow {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(rename = "statementRef", skip_serializing_if = "Option::is_none", )]
    pub statement_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statement: Option<ColabStatementModel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabStatementModel {
    pub properties: ColabModelProperties,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub acls: HashMap<ColabModelPermission, Vec<String>>,
    pub content: HashMap<String, ColabStatementElement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabStatementElement {
    #[serde(rename = "textElement")]
    pub text_element: TextElement,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub acls: HashMap<ColabModelPermission, Vec<String>>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub comments: Vec<ColabComment>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub approvals: HashMap<String, ColabUserApproval>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColabApprovalState {
    Draft,
    Pending,
    Approved,
    Rejected,
}

impl fmt::Display for ColabApprovalState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColabApprovalState::Draft => write!(f, "draft"),
            ColabApprovalState::Pending => write!(f, "pending"),
            ColabApprovalState::Approved => write!(f, "approved"),
            ColabApprovalState::Rejected => write!(f, "rejected"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabUserApproval {
    pub state: ColabApprovalState,
    pub user: uuid::Uuid,
    pub date: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabGroupApproval {
    pub state: ColabApprovalState,
    pub group: uuid::Uuid,
    pub approvals: Vec<ColabUserApproval>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ColabApproval {
    User(ColabUserApproval),
    Group(ColabGroupApproval),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColabCommentType {
    User,
}

impl fmt::Display for ColabCommentType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColabCommentType::User => write!(f, "user"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColabCommentState {
    Open,
    Resolved,
}

impl fmt::Display for ColabCommentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ColabCommentState::Open => write!(f, "open"),
            ColabCommentState::Resolved => write!(f, "resolved"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabComment {
    #[serde(rename = "type")]
    pub r#type: ColabCommentType,
    pub state: ColabCommentState,
    pub author: uuid::Uuid,
    pub text: TextElement,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextElement {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub children: Vec<TextElementChild>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub attributes: HashMap<String, String>,
    #[serde(rename = "nodeName")]
    pub node_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextElementChild {
    pub children: TextElementChildrenOrString,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub attributes: HashMap<String, String>,
    #[serde(rename = "nodeName")]
    pub node_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextElementChildrenOrString {
    AsChildren(Vec<TextElementChild>),
    AsStringArray(Vec<String>),
    AsString(String),
}

// Helper function to deserialize null as default value
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

impl<'de> Deserialize<'de> for ColabModel {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        let doc_type = value
            .get("properties")
            .and_then(|props| props.get("type"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| de::Error::missing_field("properties.type"))?;

        match doc_type {
            "colab-statement" => {
                let stmt = ColabStatementModel::deserialize(value)
                    .map_err(de::Error::custom)?;
                Ok(ColabModel::Statement(stmt))
            }
            "colab-sheet" => {
                let sheet = ColabSheetModel::deserialize(value)
                    .map_err(de::Error::custom)?;
                Ok(ColabModel::Sheet(sheet))
            }
            other => Err(de::Error::unknown_variant(other, &["colab-statement", "colab-sheet"])),
        }
    }
}