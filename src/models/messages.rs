
use serde::{Deserialize, Serialize};
use crate::models::SerializedColabDoc;

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct LoadMessage {
    pub user: String,
    pub peer: String,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UpdateMessage {
    pub delta: String,
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

#[derive(Serialize, Deserialize, Debug)]
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
pub struct BroadcastMessage {
    pub sender_id: String,
    pub content: String,
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