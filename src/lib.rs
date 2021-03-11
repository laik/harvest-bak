#![feature(proc_macro_hygiene, decl_macro)]
#![feature(trait_alias)]
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate lazy_static;

mod api;
mod event_listener;
mod server;

pub use serde_json;

pub(crate) use api::*;
pub use common::Result;
pub(crate) use event_listener::{
    DBAddEvent, DBDeleteEvent, DBUpdateEvent, ScannerCloseEvent, ScannerOpenEvent,
    ScannerWriteEvent,
};
pub use server::Harvest;

use log::error as err;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Debug, Clone)]
pub(crate) struct Rule {
    pub(crate) upload: bool,
    pub(crate) rule: String,
    pub(crate) output: String,
}

pub(crate) type RuleStorage = Arc<RwLock<HashMap<String, Rule>>>;

lazy_static! {
    static ref GLOBAL_RULES: RuleStorage = {
        let m = Arc::new(RwLock::new(HashMap::<String, Rule>::new()));
        m
    };
}

pub(crate) fn set_rule(key: String, value: Rule) {
    match GLOBAL_RULES.try_write() {
        Ok(mut db) => {
            db.insert(key, value);
        }
        Err(e) => {
            err!("{}", e);
        }
    }
}

pub(crate) fn get_rule(key: String) -> Option<Rule> {
    match GLOBAL_RULES.read() {
        Ok(db) => match db.get(&key) {
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
