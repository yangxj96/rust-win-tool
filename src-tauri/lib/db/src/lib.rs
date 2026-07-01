use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    pub id: String,
    pub data: serde_json::Value,
    pub created_at: String,
}

pub trait Database {
    fn get(&self, id: &str) -> Option<Record>;
    fn set(&mut self, record: Record);
    fn delete(&mut self, id: &str) -> bool;
}
