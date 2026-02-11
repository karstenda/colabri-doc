use loro::{LoroDoc, LoroList, LoroMap, LoroMovableList, LoroText};
use std::option::Option;
use tracing::{info};


use crate::models::{
    ColabApproval, ColabModel, ColabModelPermission, ColabSheetBlock, ColabSheetModel,
    ColabStatementModel, ColabUserApproval, TextElement, TextElementChild, TextElementChildrenOrString,
};

pub fn colab_to_loro_doc(colab_model: &ColabModel) -> Option<LoroDoc> {
    match colab_model {
        ColabModel::Statement(stmt_model) => stmt_to_loro_doc(stmt_model),
        ColabModel::Sheet(sheet_model) => sheet_to_loro_doc(sheet_model),
    }
}

pub fn sheet_to_loro_doc(sheet_model: &ColabSheetModel) -> Option<LoroDoc> {
    let loro_doc = LoroDoc::new();

    // Let's create the properties map
    let properties_loro_map = loro_doc.get_map("properties");

    // Set the type
    let _ = properties_loro_map.insert(
        "type",
        sheet_model
            .properties
            .r#type
            .to_string()
            .as_str(),
    );

    // Set the content type
    let _ = properties_loro_map.insert(
        "contentType",
        sheet_model.properties.content_type.as_str(),
    );

    // Set the masterLangCode if present
    if sheet_model.properties.master_lang_code.is_some() {
        let _ = properties_loro_map.insert(
            "masterLangCode",
            sheet_model
                .properties
                .master_lang_code
                .as_ref()
                .unwrap()
                .as_str(),
        );
    }

    // Set countryCodes if present
    info!("Setting countryCodes if present");
    if sheet_model.properties.country_codes.is_some() {
        info!("CountryCodes are present");
        let country_codes_list = properties_loro_map
            .get_or_create_container("countryCodes", LoroList::new())
            .unwrap();
        for (idx, code) in sheet_model
            .properties
            .country_codes
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
        {
            let _ = country_codes_list.insert(idx, code.as_str());
        }
    }

    // Set langCodes if present
    if sheet_model.properties.lang_codes.is_some() {
        let lang_codes_list = properties_loro_map
            .get_or_create_container("langCodes", LoroList::new())
            .unwrap();
        for (idx, code) in sheet_model
            .properties
            .lang_codes
            .as_ref()
            .unwrap()
            .iter()
            .enumerate()
        {
            let _ = lang_codes_list.insert(idx, code.as_str());
        }
    }

    // Set the ACLs (HashMap<ColabModelPermission, Vec<String>>)
    let acls_loro_map = loro_doc.get_map("acls");
    for (permission, principals) in &sheet_model.acls {
        let permission_str = permission.to_string();
        // Let's create a LoroList
        let perm_loro_list = acls_loro_map
            .get_or_create_container(&permission_str, LoroList::new())
            .unwrap();
        // Add the principals
        for (idx, principal) in principals.iter().enumerate() {
            let _ = perm_loro_list.insert(idx, principal.as_str());
        }
    }

    // Set the content in the LoroDoc from the list of content
    let content_loro_list = loro_doc.get_movable_list("content");
    for (idx, block) in sheet_model.content.iter().enumerate() {
        // Let's create a LoroMap for every block
        let block_loro_map = colab_sheet_block_to_loro_map(block);
        let _ = content_loro_list.insert_container(idx, block_loro_map);
    }
    

    // We should be done for now
    Some(loro_doc)
}

pub fn stmt_to_loro_doc(stmt_model: &ColabStatementModel) -> Option<LoroDoc> {
    let loro_doc = LoroDoc::new();

    // Let's create the properties map
    let properties_loro_map = loro_doc.get_map("properties");

    // Set the type
    let _ = properties_loro_map.insert(
        "type",
        stmt_model
            .properties
            .r#type
            .to_string()
            .as_str(),
    );
    // Set the content type
    let _ = properties_loro_map.insert(
        "contentType",
        stmt_model.properties.content_type.as_str(),
    );

    // Set the ACLs (HashMap<ColabModelPermission, Vec<String>>)
    let acls_loro_map = loro_doc.get_map("acls");
    for (permission, principals) in &stmt_model.acls {
        let permission_str = permission.to_string();
        // Let's create a LoroList
        let perm_loro_list = acls_loro_map
            .get_or_create_container(&permission_str, LoroList::new())
            .unwrap();
        // Add the principals
        for (idx, principal) in principals.iter().enumerate() {
            let _ = perm_loro_list.insert(idx, principal.as_str());
        }
    }

    // Set the content (HashMap<String, ColabStatementElement>)
    let content_loro_map = loro_doc.get_map("content");
    for (block_id, block) in &stmt_model.content {
        // Let's create a LoroMap for every block
        let block_loro_map = content_loro_map
            .get_or_create_container(block_id, LoroMap::new())
            .unwrap();

        // Set the ACLs for this Statement element (HashMap<ColabModelPermission, Vec<String>>)
        let block_acls_loro_map = block_loro_map
            .get_or_create_container("acls", LoroMap::new())
            .unwrap();
        for (permission, principals) in &block.acls {
            let permission_str = permission.to_string();
            // Let's create a LoroList
            let block_perm_loro_list = block_acls_loro_map
                .get_or_create_container(&permission_str, LoroList::new())
                .unwrap();
            // Add the principals
            for (idx, principal) in principals.iter().enumerate() {
                let _ = block_perm_loro_list.insert(idx, principal.as_str());
            }
        }

        if !block.approvals.is_empty() {
            // Mirror the approval workflow state so clients stay consistent in CRDT form.
            let approvals_loro_map = block_loro_map
                .get_or_create_container("approvals", LoroMap::new())
                .unwrap();
            for (approval_id, approval) in &block.approvals {
                let approval_loro_map = approvals_loro_map
                    .get_or_create_container(approval_id.as_str(), LoroMap::new())
                    .unwrap();
                colab_user_approval_to_loro_map(approval, &approval_loro_map);
            }
        }

        // Let's ignore comments for now.

        // Let's set the TextElement
        let text_element_loro_map = block_loro_map
            .get_or_create_container("textElement", LoroMap::new())
            .unwrap();
        txtelem_to_loro_doc(&block.text_element, &text_element_loro_map);

        // Let's set the approvals
        if !block.approvals.is_empty() {
            let approvals_loro_map = block_loro_map
                .get_or_create_container("approvals", LoroMap::new())
                .unwrap();
            for (approval_id, approval) in &block.approvals {
                let approval_loro_map = approvals_loro_map
                    .get_or_create_container(approval_id.as_str(), LoroMap::new())
                    .unwrap();
                colab_user_approval_to_loro_map(approval, &approval_loro_map);
            }
        }
    }

    // We should be done for now
    Some(loro_doc)
}

#[allow(dead_code)]
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
                let nested_list = loro_map
                    .get_or_create_container("approvals", LoroList::new())
                    .unwrap();
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
    let attributes_loro_map = loro_map
        .get_or_create_container("attributes", LoroMap::new())
        .unwrap();
    for (key, value) in &text_element.attributes {
        let _ = attributes_loro_map.insert(key, value.as_str());
    }

    // Set the children
    match &text_element.children {
        TextElementChildrenOrString::AsChildren(children_vec) => {
            let children_loro_list = loro_map
                .get_or_create_container("children", LoroList::new())
                .unwrap();
            for (idx, nested_child) in children_vec.iter().enumerate() {
                let nested_child_loro_map = LoroMap::new();
                txtelem_child_to_loro_map(
                    nested_child,
                    &nested_child_loro_map,
                    1,
                    MAX_DEPTH,
                );
                let _ = children_loro_list.insert_container(idx, nested_child_loro_map);
            }
        }
        TextElementChildrenOrString::AsStringArray(strings) => {
            let children_loro_list = loro_map
                .get_or_create_container("children", LoroList::new())
                .unwrap();
            for (idx, s) in strings.iter().enumerate() {
                let loro_text = children_loro_list
                    .insert_container(idx, LoroText::new())
                    .unwrap();
                let _ = loro_text.insert(0, s.as_str());
            }
        }
    }
}

fn txtelem_child_to_loro_map(
    child: &TextElementChild,
    loro_map: &LoroMap,
    depth: usize,
    max_depth: usize,
) {
    // Prevent stack overflow by limiting recursion depth
    if depth >= max_depth {
        let _ = loro_map.insert("nodeName", "truncated");
        let _ = loro_map.insert("children", "[Max depth exceeded]");
        return;
    }

    // Set the nodeName
    let _ = loro_map.insert("nodeName", child.node_name.as_str());

    // Set the attributes
    let attributes_loro_map = loro_map
        .get_or_create_container("attributes", LoroMap::new())
        .unwrap();
    for (key, value) in &child.attributes {
        let _ = attributes_loro_map.insert(key, value.as_str());
    }

    // Set the children
    match &child.children {
        TextElementChildrenOrString::AsChildren(children_vec) => {
            let children_loro_list = loro_map
                .get_or_create_container("children", LoroList::new())
                .unwrap();
            for (idx, nested_child) in children_vec.iter().enumerate() {
                let nested_child_loro_map = LoroMap::new();
                txtelem_child_to_loro_map(
                    nested_child,
                    &nested_child_loro_map,
                    depth + 1,
                    max_depth,
                );
                let _ = children_loro_list.insert_container(idx, nested_child_loro_map);
            }
        }
        TextElementChildrenOrString::AsStringArray(strings) => {
            let children_loro_list = loro_map
                .get_or_create_container("children", LoroList::new())
                .unwrap();
            for (idx, s) in strings.iter().enumerate() {
                let loro_text = children_loro_list
                    .insert_container(idx, LoroText::new())
                    .unwrap();
                let _ = loro_text.insert(0, s.as_str());
            }
        }
    }
}

fn colab_sheet_block_to_loro_map(block: &ColabSheetBlock) -> LoroMap {
    let loro_map = LoroMap::new();
    match block {
        ColabSheetBlock::Properties(_properties_block) => {
          let _ = loro_map.insert("type", "properties");  
        }
        ColabSheetBlock::Text(text_block) => {
            let _ = loro_map.insert("type", "text");
            // ACLs
            let acls_map = loro_map
                .insert_container("acls", LoroMap::new())
                .unwrap();
            populate_acls(&acls_map, &text_block.acls);

            // Title
            let title_element_map = loro_map
                .insert_container("title", LoroMap::new())
                .unwrap();
            txtelem_to_loro_doc(&text_block.title, &title_element_map);
            
            // TextElement
            let text_element_map = loro_map
                .insert_container("textElement", LoroMap::new())
                .unwrap();
            txtelem_to_loro_doc(&text_block.text_element, &text_element_map);
        }
        ColabSheetBlock::StatementGrid(grid_block) => {
            let _ = loro_map.insert("type", "statement-grid");
            // ACLs
            let acls_map = loro_map
                .insert_container("acls", LoroMap::new())
                .unwrap();
            populate_acls(&acls_map, &grid_block.acls);

            // Title
            let title_element_map = loro_map
                .insert_container("title", LoroMap::new())
                .unwrap();
            txtelem_to_loro_doc(&grid_block.title, &title_element_map);

            // Rows
            let rows_list = loro_map
                .insert_container("rows", LoroMovableList::new())
                .unwrap();
            
            for (idx, row) in grid_block.rows.iter().enumerate() {
                let row_map = LoroMap::new();
                let _ = row_map.insert("type", row.r#type.as_str());
                
                if let Some(s) = &row.statement_ref {
                    let statement_ref_map = row_map
                        .insert_container("statementRef", LoroMap::new())
                        .unwrap();
                    let _ = statement_ref_map.insert(
                        "docId",
                        s.doc_id.to_string().as_str(),
                    );
                    let _ = statement_ref_map.insert(
                        "version",
                        s.version,
                    );
                    let _ = statement_ref_map.insert(
                        "versionV",
                        s.version_v.as_str(),
                    );
                }

                if let Some(stmt) = &row.statement {
                    let statement_map = row_map
                        .insert_container("statement", LoroMap::new())
                        .unwrap();
                    stmt_to_loro_map(stmt, &statement_map);
                }

                let _ = rows_list.insert_container(idx, row_map);
            }
        }
        ColabSheetBlock::Attributes(attribute_block) => {
            let _ = loro_map.insert("type", "attributes");
            // ACLs
            let acls_map = loro_map
                .insert_container("acls", LoroMap::new())
                .unwrap();
            populate_acls(&acls_map, &attribute_block.acls);

            // Title
            let title_element_map = loro_map
                .insert_container("title", LoroMap::new())
                .unwrap();
            txtelem_to_loro_doc(&attribute_block.title, &title_element_map);

            // Attributes
            let attributes_map = loro_map
                .insert_container("attributes", LoroMap::new())
                .unwrap();
            for (key, value) in &attribute_block.attributes {
                // Serialize the attribute value to a string.
                let value_json = serde_json::to_string(value).unwrap_or_else(|_| "".to_string());
                let _ = attributes_map.insert(key, value_json.as_str());
            }
        }
    }
    loro_map
}

fn stmt_to_loro_map(stmt_model: &ColabStatementModel, loro_map: &LoroMap) {
    // Properties
    let properties_map = loro_map.insert_container("properties", LoroMap::new()).unwrap();
    let _ = properties_map.insert("type", stmt_model.properties.r#type.to_string().as_str());
    let _ = properties_map.insert("contentType", stmt_model.properties.content_type.as_str());

    // ACLs
    let acls_map = loro_map.insert_container("acls", LoroMap::new()).unwrap();
    populate_acls(&acls_map, &stmt_model.acls);

    // Content
    let content_map = loro_map.insert_container("content", LoroMap::new()).unwrap();
    for (block_id, block) in &stmt_model.content {
        let block_loro_map = content_map
            .get_or_create_container(block_id, LoroMap::new())
            .unwrap();
        
        // Block ACLs
        let block_acls_loro_map = block_loro_map
            .get_or_create_container("acls", LoroMap::new())
            .unwrap();
        populate_acls(&block_acls_loro_map, &block.acls);

        // Approvals
        if !block.approvals.is_empty() {
            let approvals_loro_map = block_loro_map
                .get_or_create_container("approvals", LoroMap::new())
                .unwrap();
            for (approval_id, approval) in &block.approvals {
                let approval_loro_map = approvals_loro_map
                    .get_or_create_container(approval_id.as_str(), LoroMap::new())
                    .unwrap();
                colab_user_approval_to_loro_map(approval, &approval_loro_map);
            }
        }

        // TextElement
        let text_element_loro_map = block_loro_map
            .get_or_create_container("textElement", LoroMap::new())
            .unwrap();
        txtelem_to_loro_doc(&block.text_element, &text_element_loro_map);
    }
}

fn populate_acls(acls_map: &LoroMap, acls: &std::collections::HashMap<ColabModelPermission, Vec<String>>) {
    for (permission, principals) in acls {
        let permission_str = permission.to_string();
        let perm_loro_list = acls_map
            .get_or_create_container(&permission_str, LoroList::new())
            .unwrap();
        for (idx, principal) in principals.iter().enumerate() {
            let _ = perm_loro_list.insert(idx, principal.as_str());
        }
    }
}
