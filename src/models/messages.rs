
use serde::{Deserialize, Serialize};
use crate::models::SerializedColabDoc;
use serde_with::{serde_as, base64::Base64};

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoadMessage {
    pub user: String,
    pub peer: String,
}

#[serde_as]
#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMessage {
    #[serde_as(as = "Base64")]
    pub delta: Vec<u8>,
    pub user: String,
    pub peer: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct PingMessage {
    pub user: String,
    pub peer: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct InitMessage {
    pub colab_doc: SerializedColabDoc,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PongMessage {
    pub date: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum ReceivedMessage {
    #[serde(rename = "load")]
    Load(LoadMessage),
    #[serde(rename = "update")]
    Update(UpdateMessage),
    #[serde(rename = "ping")]
    Ping(PingMessage),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct BroadcastUpdateMessage {
    pub sender_id: String,
    pub update: UpdateMessage,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum SendMessage {
    #[serde(rename = "init")]
    Init(InitMessage),
    #[serde(rename = "update")]
    Update(UpdateMessage),
    #[serde(rename = "pong")]
    Pong(PongMessage),
}