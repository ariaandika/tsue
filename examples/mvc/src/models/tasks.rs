use serde::{Deserialize, Serialize};

#[derive(Clone,Serialize,Deserialize)]
pub struct Tasks {
    pub id: usize,
    pub name: String,
}

#[derive(Debug, Deserialize)]
pub struct AddTask {
    pub name: String,
}

