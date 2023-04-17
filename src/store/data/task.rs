use super::DbModel;
use crate::TaskState;
use acts_tag::{Tags, Value};
use serde::{Deserialize, Serialize};

#[derive(Tags, Debug, Deserialize, Serialize, Clone)]
pub struct Task {
    #[tag(id)]
    pub id: String,
    #[tag]
    pub pid: String,
    #[tag]
    pub tid: String,
    #[tag]
    pub nid: String,
    #[tag]
    pub uid: String,
    pub kind: String,
    pub state: String,
    pub start_time: i64,
    pub end_time: i64,
}

impl Task {
    pub fn set_state(&mut self, state: TaskState) {
        self.state = state.into();
    }
    pub fn set_start_time(&mut self, time: i64) {
        self.start_time = time;
    }
    pub fn set_end_time(&mut self, time: i64) {
        self.end_time = time;
    }
    pub fn set_user(&mut self, user: &str) {
        self.uid = user.to_string();
    }
}

impl DbModel for Task {
    fn id(&self) -> &str {
        &self.id
    }
}
