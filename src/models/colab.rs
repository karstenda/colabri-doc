use std::collections::HashMap;
use std::option::Option;
use std::fmt;
use loro::{LoroDoc, LoroList, LoroMap, LoroText};
use serde::{Deserialize, Deserializer, Serialize};
use chrono::{DateTime, Utc};

// Helper function to deserialize null as default value
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabModel {
    pub properties: ColabModelProperties,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabModelProperties {
    #[serde(rename = "type")]
    pub r#type: ColabModelType,
    #[serde(rename = "contentType")]
    pub content_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColabStatementModel {
    #[serde(flatten)]
    pub colab_model: ColabModel,
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
    pub approvals: Vec<ColabApproval>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColabApprovalState {
    Draft,
    Pending,
    Approved,
    Rejected
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
    pub children: Vec<TextElementChild>,
    pub attributes: HashMap<String, String>,
    #[serde(rename = "nodeName")]
    pub node_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextElementChild {
    pub children: TextElementChildrenOrString,
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

pub fn stmt_to_loro_doc(stmt_model: &ColabStatementModel) -> Option<LoroDoc> {
    
    let loro_doc = LoroDoc::new();

    // Let's create the properties map
    let properties_loro_map = loro_doc.get_map("properties");

    // Set the type
    let _ = properties_loro_map.insert("type", stmt_model.colab_model.properties.r#type.to_string().as_str());
    // Set the content type
    let _ = properties_loro_map.insert("contentType", stmt_model.colab_model.properties.content_type.as_str());

    // Set the ACLs (HashMap<ColabModelPermission, Vec<String>>)
    let acls_loro_map = loro_doc.get_map("acls");
    for (permission, principals) in &stmt_model.acls {
        let permission_str = permission.to_string();
        // Let's create a LoroList
        let perm_loro_list = acls_loro_map.get_or_create_container(&permission_str, LoroList::new()).unwrap();
        // Add the principals
        for (idx, principal) in principals.iter().enumerate() {
            let _ = perm_loro_list.insert(idx, principal.as_str());
        }
    }

    // Set the content (HashMap<String, ColabStatementElement>)
    let content_loro_map = loro_doc.get_map("content");
    for (block_id, block) in &stmt_model.content {

        // Let's create a LoroMap for every block
        let block_loro_map = content_loro_map.get_or_create_container(block_id, LoroMap::new()).unwrap();

        // Set the ACLs for this Statement element (HashMap<ColabModelPermission, Vec<String>>)
        let block_acls_loro_map = block_loro_map.get_or_create_container("acls", LoroMap::new()).unwrap();
        for (permission, principals) in &block.acls {
            let permission_str = permission.to_string();
            // Let's create a LoroList
            let block_perm_loro_list = block_acls_loro_map.get_or_create_container(&permission_str, LoroList::new()).unwrap();
            // Add the principals
            for (idx, principal) in principals.iter().enumerate() {
                let _ =block_perm_loro_list.insert(idx, principal.as_str());
            }
        }

        if !block.approvals.is_empty() {
            // Mirror the approval workflow state so clients stay consistent in CRDT form.
            let approvals_loro_list = block_loro_map.get_or_create_container("approvals", LoroList::new()).unwrap();
            for (idx, approval) in block.approvals.iter().enumerate() {
                let approval_loro_map = LoroMap::new();
                colab_approval_to_loro_map(approval, &approval_loro_map);
                let _ = approvals_loro_list.insert_container(idx, approval_loro_map);
            }
        }

        // Let's ignore comments for now.

        // Let's set the TextElement
        let text_element_loro_map = block_loro_map.get_or_create_container("textElement", LoroMap::new()).unwrap();
        txtelem_to_loro_doc(&block.text_element, &text_element_loro_map);
    }

    // We should be done for now
    Some(loro_doc)
}

fn colab_approval_to_loro_map(approval: &ColabApproval, loro_map: &LoroMap) {
    match approval {
        ColabApproval::User(user_approval) => {
            let _ = loro_map.insert("type", "user");
            colab_user_approval_to_loro_map(user_approval, loro_map);
        }
        ColabApproval::Group(group_approval) => {
            let _ = loro_map.insert("type", "group");
            let state_str = group_approval.state.to_string();
            let _ = loro_map.insert("state", state_str.as_str());

            let group_str = group_approval.group.to_string();
            let _ = loro_map.insert("group", group_str.as_str());

            if !group_approval.approvals.is_empty() {
                let nested_list = loro_map.get_or_create_container("approvals", LoroList::new()).unwrap();
                for (idx, nested_approval) in group_approval.approvals.iter().enumerate() {
                    let nested_map = LoroMap::new();
                    colab_user_approval_to_loro_map(nested_approval, &nested_map);
                    let _ = nested_list.insert_container(idx, nested_map);
                }
            }
        }
    }
}

fn colab_user_approval_to_loro_map(user_approval: &ColabUserApproval, loro_map: &LoroMap) {
    let state_str = user_approval.state.to_string();
    let _ = loro_map.insert("state", state_str.as_str());

    let user_str = user_approval.user.to_string();
    let _ = loro_map.insert("user", user_str.as_str());

    let date_str = user_approval.date.to_rfc3339();
    let _ = loro_map.insert("date", date_str.as_str());
}

fn txtelem_to_loro_doc(text_element: &TextElement, loro_map: &LoroMap) {
    const MAX_DEPTH: usize = 100; // Prevent stack overflow
    
    // Set the nodeName
    let _ = loro_map.insert("nodeName", text_element.node_name.as_str());

    // Set the attributes
    let attributes_loro_map = loro_map.get_or_create_container("attributes", LoroMap::new()).unwrap();
    for (key, value) in &text_element.attributes {
        let _ = attributes_loro_map.insert(key, value.as_str());
    }

    // Set the children
    let children_loro_list = loro_map.get_or_create_container("children", LoroList::new()).unwrap();
    for (idx, child) in text_element.children.iter().enumerate() {
        let child_loro_map = LoroMap::new();
        txtelem_child_to_loro_map(child, &child_loro_map, 0, MAX_DEPTH);
        let _ = children_loro_list.insert_container(idx, child_loro_map);
    }
}

fn txtelem_child_to_loro_map(child: &TextElementChild, loro_map: &LoroMap, depth: usize, max_depth: usize) {
    // Prevent stack overflow by limiting recursion depth
    if depth >= max_depth {
        let _ = loro_map.insert("nodeName", "truncated");
        let _ = loro_map.insert("children", "[Max depth exceeded]");
        return;
    }
    
    // Set the nodeName
    let _ = loro_map.insert("nodeName", child.node_name.as_str());

    // Set the attributes
    let attributes_loro_map = loro_map.get_or_create_container("attributes", LoroMap::new()).unwrap();
    for (key, value) in &child.attributes {
        let _ = attributes_loro_map.insert(key, value.as_str());
    }

    // Set the children
    match &child.children {
        TextElementChildrenOrString::AsChildren(children_vec) => {
            let children_loro_list = loro_map.get_or_create_container("children", LoroList::new()).unwrap();
            for (idx, nested_child) in children_vec.iter().enumerate() {
                let nested_child_loro_map = LoroMap::new();
                txtelem_child_to_loro_map(nested_child, &nested_child_loro_map, depth + 1, max_depth);
                let _ = children_loro_list.insert_container(idx, nested_child_loro_map);
            }
        }
        TextElementChildrenOrString::AsStringArray(strings) => {
            let children_loro_list = loro_map.get_or_create_container("children", LoroList::new()).unwrap();
            for (idx, s) in strings.iter().enumerate() {
                let loro_text = children_loro_list.insert_container(idx, LoroText::new()).unwrap();
                let _ = loro_text.insert(0, s.as_str());
            }
        }
        TextElementChildrenOrString::AsString(s) => {
            let children_loro_list = loro_map.get_or_create_container("children", LoroList::new()).unwrap();
            let loro_text = children_loro_list.insert_container(0, LoroText::new()).unwrap();
            let _ = loro_text.insert(0, s.as_str());
        }
    }
}