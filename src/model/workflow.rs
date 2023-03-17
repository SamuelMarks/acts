use super::Job;
use crate::{utils, ActError, ActResult, ModelBase, Vars};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Workflow {
    #[serde(default)]
    pub id: String,

    #[serde(default)]
    pub ver: String,

    #[serde(default)]
    pub name: String,

    #[serde(default)]
    pub jobs: Vec<Job>,

    #[serde(default)]
    pub env: HashMap<String, Value>,

    #[serde(default)]
    pub outputs: HashMap<String, Value>,

    #[serde(default)]
    pub on: HashMap<String, Value>,

    #[serde(default)]
    pub(crate) biz_id: String,
}

impl Workflow {
    pub fn from_str(s: &str) -> ActResult<Self> {
        let workflow = serde_yaml::from_str::<Workflow>(s);
        match workflow {
            Ok(v) => Ok(v),
            Err(e) => Err(ActError::ParseError(format!("{}", e))),
        }
    }

    pub fn set_env(&mut self, vars: Vars) {
        for (name, value) in vars {
            self.env
                .entry(name)
                .and_modify(|v| *v = value.clone())
                .or_insert(value);
        }
    }

    pub fn print_tree(&self) -> ActResult<()> {
        utils::log::print_tree(&self)
    }

    pub fn job(&self, id: &str) -> Option<&Job> {
        match self.jobs.iter().find(|job| job.id == id) {
            Some(job) => {
                // job.set_workflow(Box::new(self.clone()));
                Some(job)
            }
            None => None,
        }
    }

    pub fn set_id(&mut self, id: &str) {
        self.id = id.to_string();
    }

    pub fn biz_id(&self) -> String {
        self.biz_id.clone()
    }

    pub fn set_biz_id(&mut self, biz_id: &str) {
        self.biz_id = biz_id.to_string();
    }

    pub fn to_string<'a>(&self) -> ActResult<String> {
        match serde_yaml::to_string(self) {
            Ok(s) => Ok(s),
            Err(e) => Err(ActError::ParseError(e.to_string())),
        }
    }
}

impl ModelBase for Workflow {
    fn id(&self) -> &str {
        &self.id
    }
}
