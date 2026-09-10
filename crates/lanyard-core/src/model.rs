use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
pub type Fields = BTreeMap<String, String>;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    ApiKey,
    Password,
    License,
    Recovery,
    Env,
    Crypto,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    pub id: String,
    pub title: String,
    pub description: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub title: String,
    pub category: Category,
    pub project_id: Option<String>,
    pub fields: Vec<String>,
    pub(crate) secret: SecretRef,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct SecretRef {
    pub id: String,
    pub chunks: usize,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Client {
    pub id: String,
    pub name: String,
    pub(crate) token_hash: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Grant {
    pub client_id: String,
    pub item_id: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Wrap {
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct State {
    pub schema: u32,
    pub wrap: Option<Wrap>,
    pub projects: Vec<Project>,
    pub items: Vec<Item>,
    pub clients: Vec<Client>,
    pub grants: Vec<Grant>,
    pub close_to_tray: bool,
    pub mac: String,
}
impl Default for State {
    fn default() -> Self {
        Self {
            schema: 2,
            wrap: None,
            projects: vec![],
            items: vec![],
            clients: vec![],
            grants: vec![],
            close_to_tray: true,
            mac: String::new(),
        }
    }
}
#[derive(Debug, Clone, Serialize)]
pub struct ClientInfo {
    pub id: String,
    pub name: String,
}
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot {
    pub initialized: bool,
    pub locked: bool,
    pub projects: Vec<Project>,
    pub items: Vec<Item>,
    pub clients: Vec<ClientInfo>,
    pub grants: Vec<Grant>,
    pub close_to_tray: bool,
}
#[derive(Debug, Clone, Deserialize)]
pub struct ItemInput {
    pub id: Option<String>,
    pub title: String,
    pub category: Category,
    pub project_id: Option<String>,
    pub fields: Fields,
}
#[derive(Clone, Deserialize)]
pub struct ClientAuth {
    pub client_id: String,
    pub app_name: String,
    pub token: String,
}
impl std::fmt::Debug for ClientAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientAuth")
            .field("client_id", &self.client_id)
            .finish_non_exhaustive()
    }
}
