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
pub(crate) use handle::{
    DBCloseEvent, DBOpenEvent, ScannerCloseEvent, ScannerOpenEvent, ScannerWriteEvent,
};
pub use server::Harvest;

use crossbeam_channel::{unbounded, Sender};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use std::{collections::HashMap, thread};

type TaskList = Vec<(String, Task)>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TaskListMarshaller(TaskList);

impl TaskListMarshaller {
    pub fn to_json(&self) -> String {
        match serde_json::to_string(&self.0) {
            Ok(contents) => contents,
            Err(_) => "".to_owned(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct Task {
    pub(crate) ns: String,
    pub(crate) service_name: String,
    pub(crate) pod_name: String,
    pub(crate) upload: bool,
    pub(crate) rule: String,
    pub(crate) output: String,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            upload: false,
            rule: "".to_string(),
            service_name: "".to_string(),
            pod_name: "".to_string(),
            ns: "".to_string(),
            output: "".to_string(),
        }
    }
}

#[derive(Debug)]
enum TaskMessage {
    Apply(Task),
    Close,
}

pub(crate) struct TaskStorage {
    tasks: Arc<RwLock<HashMap<String, Task>>>,
    // internal event send queue
    tx: Sender<TaskMessage>,
}

impl TaskStorage {
    pub fn new() -> Self {
        let tasks = Arc::new(RwLock::new(HashMap::<String, Task>::new()));
        let (tx, rx) = unbounded::<TaskMessage>();

        let thread_tasks = Arc::clone(&tasks);
        thread::spawn(move || {
            while let Ok(task_message) = rx.recv() {
                match task_message {
                    TaskMessage::Apply(task) => {
                        let mut tasks = match thread_tasks.write() {
                            Ok(it) => it,
                            Err(e) => {
                                eprintln!("{}", e);
                                continue;
                            }
                        };
                        tasks.entry(task.pod_name.clone()).or_insert(task);
                    }
                    TaskMessage::Close => {
                        return;
                    }
                }
            }
        });
        Self { tasks, tx }
    }
}

lazy_static! {
    static ref TASKS: TaskStorage = {
        let t = TaskStorage::new();
        t
    };
}

pub(crate) fn run_task(task: &Task) {}
pub(crate) fn stop_task(task: &Task) {}

pub(crate) fn tasks_json<'a>() -> &'a str {
    ""
}
pub(crate) fn get_task(task: &Task) {}

// pub(crate) fn set_rule(key: &str, task: Task) {
//     match TASKS.write() {
//         Ok(mut db) => {
//             db.insert(key.to_string(), task);
//         }
//         Err(e) => {
//             err!("{}", e);
//         }
//     }
// }

// pub(crate) fn tasks_json() -> String {
//     if let Ok(db) = TASKS.read() {
//         return TaskListMarshaller(
//             db.iter()
//                 .map(|(k, v)| (k.clone(), v.clone()))
//                 .collect::<Vec<(String, Task)>>(),
//         )
//         .to_json();
//     }
//     "".into()
// }

// pub(crate) fn apply_tasks() {
//     if let Ok(hm) = TASKS.read() {
//         for (pod_name, rule) in hm.iter() {
//             let mut result_pod_list = vec![];
//             for (_, mut v) in db::get_slice_with_ns_pod(&rule.ns, pod_name) {
//                 if rule.upload {
//                     v.upload();
//                 }
//                 result_pod_list.push(v.clone());
//             }
//             for p in result_pod_list {
//                 db::apply(&p);
//             }
//         }
//     }
// }

// pub(crate) fn get_task(key: &str) -> Option<Task> {
//     match TASKS.read() {
//         Ok(db) => match db.get(key) {
//             Some(task) => Some(task.clone()),
//             None => None,
//         },
//         Err(e) => {
//             err!("{}", e);
//             None
//         }
//     }
// }

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
