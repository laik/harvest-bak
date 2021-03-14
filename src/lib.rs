#![feature(proc_macro_hygiene, decl_macro)]
#![feature(trait_alias)]
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate lazy_static;

mod api;
mod handle;
mod server;

pub use serde_json;

pub(crate) use api::*;
pub use common::Result;
pub(crate) use handle::{ScannerCloseEvent, ScannerOpenEvent, ScannerWriteEvent};
pub use server::Harvest;

use log::error as err;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

type RuleList = Vec<(String, Rule)>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RuleListMarshaller(RuleList);

impl RuleListMarshaller {
    pub fn to_json(&self) -> String {
        match serde_json::to_string(&self.0) {
            Ok(contents) => contents,
            Err(_) => "".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Rule {
    pub(crate) upload: bool,
    pub(crate) rule: String,
    pub(crate) service_name: String,
    pub(crate) ns: String,
    pub(crate) output: String,
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            upload: false,
            rule: "".into(),
            service_name: "".into(),
            ns: "".into(),
            output: "".into(),
        }
    }
}

pub(crate) type RuleStorage = Arc<RwLock<HashMap<String, Rule>>>;

lazy_static! {
    static ref GLOBAL_RULES: RuleStorage = {
        let m = Arc::new(RwLock::new(HashMap::<String, Rule>::new()));
        m
    };
}

pub(crate) fn set_rule(key: &str, value: Rule) {
    match GLOBAL_RULES.write() {
        Ok(mut db) => {
            db.insert(key.to_string(), value);
        }
        Err(e) => {
            err!("{}", e);
        }
    }
}

pub(crate) fn rules_json() -> String {
    if let Ok(db) = GLOBAL_RULES.read() {
        return RuleListMarshaller(
            db.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect::<Vec<(String, Rule)>>(),
        )
        .to_json();
    }
    "".into()
}

pub(crate) fn apply_rules() {
    if let Ok(hm) = GLOBAL_RULES.read() {
        for (pod_name, rule) in hm.iter() {
            let mut result_pod_list = vec![];
            for (_, mut v) in db::get_slice_with_ns_pod(&rule.ns, pod_name) {
                if rule.upload {
                    v.upload();
                }
                result_pod_list.push(v.clone());
            }
            for p in result_pod_list {
                db::apply(&p);
            }
        }
    }
}

pub(crate) fn get_rule(key: &str) -> Option<Rule> {
    match GLOBAL_RULES.read() {
        Ok(db) => match db.get(key) {
            Some(rule) => Some(rule.clone()),
            None => None,
        },
        Err(e) => {
            err!("{}", e);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
