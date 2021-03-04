// #![feature(str_split_once)]

use common::Result;
use db::{Database, Pod};
use event::obj::Listener;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::{
    collections::HashMap,
    sync::{mpsc::channel, Arc, Mutex},
    time::Duration,
};
use strum::AsRefStr;
use walkdir::WalkDir;

#[derive(Debug, AsRefStr, Clone)]
pub enum PathEvent {
    #[strum(serialize = "NeedOpen")]
    NeedOpen,
    #[strum(serialize = "NeedClose")]
    NeedClose,
    #[strum(serialize = "NeedWrite")]
    NeedWrite,
}

pub trait GetPathEventInfo {
    fn get(&self) -> Option<&PathEventInfo>;
}

#[derive(Debug, Clone)]
pub struct PathEventInfo {
    pub(crate) namespace: String,
    pub(crate) pod: String,
    pub(crate) container: String,
    pub path: String,
}

impl GetPathEventInfo for PathEventInfo {
    fn get(&self) -> Option<&PathEventInfo> {
        Some(self)
    }
}

impl PathEventInfo {
    pub fn to_pod(&self) -> Pod {
        Pod {
            uuid: self.path.clone(),
            namespace: self.namespace.clone(),
            pod_name: self.pod.clone(),
            container_name: self.container.clone(),
            ..Default::default()
        }
    }
}

unsafe impl Sync for PathEventInfo {}
unsafe impl Send for PathEventInfo {}

pub type ScannerRecvArgument = (PathEventInfo, Arc<Mutex<Database>>);

impl GetPathEventInfo for ScannerRecvArgument {
    fn get(&self) -> Option<&PathEventInfo> {
        Some(&self.0)
    }
}

#[derive(Clone)]
pub struct AutoScanner {
    namespace: String,
    dir: String,
    db: Arc<Mutex<Database>>,
}

impl AutoScanner {
    pub fn new(namespace: String, dir: String, db: Arc<Mutex<Database>>) -> Self {
        Self { namespace, dir, db }
    }

    // TODO
    // the path eg:
    // /var/log/pod
    //default_mysql-apollo-slave-0_49d0b6e1-9980-4f7b-b1eb-3eab3e753b48
    // └── mysql
    //     ├── 4.log -> /data/docker/containers/1707c92da3df11616bd8eb15bf1c8e60105e5276b62acba3c0aa12e3d0f03df5/1707c92da3df11616bd8eb15bf1c8e60105e5276b62acba3c0aa12e3d0f03df5-json.log
    //     └── 5.log -> /data/docker/containers/5b3c5c7cd28f42a3e5320c8f0e64988de2ddeb6f87223c833045f1e0fcf74528/5b3c5c7cd28f42a3e5320c8f0e64988de2ddeb6f87223c833045f1e0fcf74528-json.log
    // expect:
    // ParsePodForPath{
    //  pod: mysql-apollo-slave-0
    //  namespace: default
    //  container: mysql
    //  logfiles: [4.log,5.log]
    // }
    // /var/log/pods/default_mysql-apollo-slave-0_49d0b6e1-9980-4f7b-b1eb-3eab3e753b48/mysql/4.log
    fn parse_path_to_pei(namespace: String, dir: String, path: String) -> Option<PathEventInfo> {
        if !path.starts_with(&dir) || !path.ends_with(".log") {
            return None;
        }

        // /default_mysql-apollo-slave-0_49d0b6e1-9980-4f7b-b1eb-3eab3e753b48/mysql/4.log
        let (_, ns_pod_uuid_container_log) =
            path.strip_prefix(&dir).unwrap().split_once("/").unwrap();

        // default_mysql-apollo-slave-0_49d0b6e1-9980-4f7b-b1eb-3eab3e753b48 mysql/4.log
        let (ns_pod_uuid, remain) = ns_pod_uuid_container_log.split_once("/").unwrap();

        // ["default","mysql-apollo-slave-0","49d0b6e1-9980-4f7b-b1eb-3eab3e753b48"]
        let ns_pod_list = ns_pod_uuid.split("_").collect::<Vec<&str>>();
        if ns_pod_list.len() < 3 || namespace != ns_pod_list[0].to_string() {
            return None;
        }

        // container: mysql
        let container = remain.split("/").collect::<Vec<&str>>()[0];

        Some(PathEventInfo {
            namespace: ns_pod_list[0].to_string(),
            pod: ns_pod_list[1].to_string(),
            container: container.to_owned(),
            path: path.clone(),
        })
    }

    fn prepare_scanner(&self) -> Result<()> {
        for entry in WalkDir::new(self.dir.clone()) {
            let entry = entry?;
            if !entry.path().is_file() {
                continue;
            }
            let pei = Self::parse_path_to_pei(
                self.namespace.clone(),
                self.dir.clone(),
                entry.path().to_str().unwrap().to_owned(),
            )
            .unwrap();

            match self.db.lock() {
                Ok(mut _db) => {
                    _db.put(pei.to_pod())?;
                }
                Err(e) => {
                    eprintln!("auto_scanner start prepare_scanner error: {}", e);
                    continue;
                }
            }

            continue;
        }
        Ok(())
    }

    pub fn start(
        &self,
        handles: Arc<
            Mutex<HashMap<String, Box<dyn Listener<ScannerRecvArgument> + Send + 'static>>>,
        >,
    ) -> Result<()> {
        println!("🔧 harvest auto_scanner start prepare_scanner!!!");
        // 首先list符合namespace的目录，然后watch目录的变更
        self.prepare_scanner()?;

        println!("🔧 harvest auto_scanner start");
        // Create a channel to receive the events.
        let (tx, rx) = channel();

        // Create a watcher object, delivering debounced events.
        // The notification back-end is selected based on the platform.
        let mut watcher = watcher(tx, Duration::from_millis(0))?;

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        if let Err(e) = watcher.watch(&self.dir, RecursiveMode::Recursive) {
            return Err(Box::new(e));
        }
        let handles_arc_clone = handles.clone();
        let handles_lock = handles_arc_clone.lock().unwrap();
        loop {
            match rx.recv() {
                Ok(event) => match event {
                    DebouncedEvent::Create(path) => {
                        if let Some(path_str) = path.to_str() {
                            match Self::parse_path_to_pei(
                                self.namespace.to_owned(),
                                self.dir.clone(),
                                path_str.to_owned(),
                            ) {
                                Some(pei) => {
                                    if let Some(o) = handles_lock.get(PathEvent::NeedOpen.as_ref())
                                    {
                                        o.handle((pei, self.db.clone()))
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    DebouncedEvent::Write(path) => {
                        if let Some(path_str) = path.to_str() {
                            match Self::parse_path_to_pei(
                                self.namespace.to_owned(),
                                self.dir.clone(),
                                path_str.to_owned(),
                            ) {
                                Some(pei) => {
                                    if let Some(o) = handles_lock.get(PathEvent::NeedWrite.as_ref())
                                    {
                                        o.handle((pei, self.db.clone()))
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    DebouncedEvent::Remove(path) => {
                        if let Some(path_str) = path.to_str() {
                            match Self::parse_path_to_pei(
                                self.namespace.to_owned(),
                                self.dir.clone(),
                                path_str.to_owned(),
                            ) {
                                Some(pei) => {
                                    if let Some(o) = handles_lock.get(PathEvent::NeedClose.as_ref())
                                    {
                                        o.handle((pei, self.db.clone()))
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    DebouncedEvent::Error(e, path) => {
                        println!("watch event error: {:?} option: {:?}", e, path);
                    }
                    _ => {}
                },
                Err(e) => {
                    println!("watch error: {:?}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use crate::{AutoScanner, PathEvent, PathEventInfo};
    use db::Database;
    use event::obj::{Dispatch, Listener};

    #[test]
    fn it_works() {
        trait Handle: Sync + Send {
            fn handle(&self, pei: PathEventInfo, db: Arc<Mutex<Database>>) {
                println!("mock handle pei {:?}", pei);
            }
        }
        let maps = Arc::new(HashMap::<String, Arc<dyn Handle>>::new());

        let datanase = Arc::new(Mutex::new(Database::new(Dispatch::new())));
        let auto_scanner = AutoScanner::new("default".to_owned(), "".to_owned(), datanase.clone());

        // new a event_handler registry mock action events handle
        let mut path_event_handler = Dispatch::<(PathEventInfo, Arc<Mutex<Database>>)>::new();

        struct ListenerImpl;
        impl<T> Listener<T> for ListenerImpl
        where
            T: Clone,
        {
            fn handle(&self, t: T) {
                // println!("{:?}", t);
            }
        }

        path_event_handler.registry(PathEvent::NeedClose.as_ref().to_owned(), ListenerImpl);
    }

    #[test]
    fn event_it_works() {
        assert_eq!(PathEvent::NeedClose.as_ref(), "NeedClose");
        assert_eq!(PathEvent::NeedOpen.as_ref(), "NeedOpen");
        assert_eq!(PathEvent::NeedWrite.as_ref(), "NeedWrite");
    }
}
