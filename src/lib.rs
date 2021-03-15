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

use db::Pod;
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

lazy_static! {
    static ref TASKS: TaskStorage = {
        let task_storage = TaskStorage::new();
        task_storage
    };
}

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
    pod: Pod,
}

impl<'a> From<RequestPod<'a>> for Task {
    fn from(a: RequestPod) -> Self {
        let ips = a
            .ips
            .iter()
            .map(|ip| ip.to_string())
            .collect::<Vec<String>>();
        Self {
            pod: Pod {
                pod_name: a.pod.to_string(),
                offset: a.offset,
                ips,
                ..Default::default()
            },
        }
    }
}

impl Default for Task {
    fn default() -> Self {
        Self {
            pod: Pod::default(),
        }
    }
}

#[derive(Debug)]
enum TaskMessage {
    Run(Task),
    Stop(Task),
    Close,
}

pub(crate) struct TaskStorage {
    data: Arc<RwLock<HashMap<String, Task>>>,
    // internal event send queue
    tx: Sender<TaskMessage>,
}

impl TaskStorage {
    pub fn new() -> Self {
        let data = Arc::new(RwLock::new(HashMap::<String, Task>::new()));
        let (tx, rx) = unbounded::<TaskMessage>();

        let thread_tasks = Arc::clone(&data);
        thread::spawn(move || {
            while let Ok(task_message) = rx.recv() {
                match task_message {
                    TaskMessage::Close => {
                        return;
                    }
                    TaskMessage::Run(mut task) => {
                        let mut tasks = match thread_tasks.write() {
                            Ok(it) => it,
                            Err(e) => {
                                eprintln!("{}", e);
                                continue;
                            }
                        };
                        task.pod.upload();
                        tasks.entry(task.pod.pod_name.clone()).or_insert(task);
                    }
                    TaskMessage::Stop(mut task) => {
                        let mut tasks = match thread_tasks.write() {
                            Ok(it) => it,
                            Err(e) => {
                                eprintln!("{}", e);
                                continue;
                            }
                        };
                        task.pod.un_upload();
                        tasks.entry(task.pod.pod_name.clone()).or_insert(task);
                    }
                }
            }
        });
        Self { data, tx }
    }
}

pub(crate) fn run_task(task: &Task) {
    TASKS.tx.send(TaskMessage::Run(task.clone())).unwrap();
}

pub(crate) fn stop_task(task: &Task) {
    TASKS.tx.send(TaskMessage::Stop(task.clone())).unwrap();
}

pub(crate) fn task_close() {
    TASKS.tx.send(TaskMessage::Close).unwrap();
}

pub(crate) fn tasks_json() -> String {
    if let Ok(tasks) = TASKS.data.read() {
        let task_list = tasks
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect::<Vec<(String, Task)>>();

        return TaskListMarshaller(task_list).to_json();
    }
    "".to_string()
}

pub(crate) fn get_pod_task(pod_name: &str) -> Option<Task> {
    match TASKS.data.read() {
        Ok(db) => match db.get(pod_name) {
            Some(task) => Some(task.clone()),
            None => None,
        },
        Err(e) => {
            eprintln!("{}", e);
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
