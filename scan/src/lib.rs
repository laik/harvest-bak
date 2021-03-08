use common::Result;
use db::{Database, Pod};
use event::{obj::Listener, Dispatch};
use notify::{raw_watcher, RawEvent, RecursiveMode, Watcher};
use std::sync::RwLock;
use std::sync::{mpsc::channel, Arc};
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

pub trait GetDebug {
    fn get_debug(&self) -> String;
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

impl GetDebug for PathEventInfo {
    fn get_debug(&self) -> String {
        format!("{:?}", self).to_owned()
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

pub type ScannerRecvArgument = (PathEventInfo, Arc<RwLock<Database>>);

impl GetPathEventInfo for ScannerRecvArgument {
    fn get(&self) -> Option<&PathEventInfo> {
        Some(&self.0)
    }
}

pub struct AutoScanner {
    namespace: String,
    dir: String,
    event_dispatch: Dispatch<PathEventInfo>,
}

impl AutoScanner {
    pub fn new(namespace: String, dir: String) -> Self {
        Self {
            namespace,
            dir,
            event_dispatch: Dispatch::<PathEventInfo>::new(),
        }
    }

    pub fn append_close_event_handle<L>(&mut self, l: L)
    where
        L: Listener<PathEventInfo> + Send + Sync + 'static,
    {
        self.event_dispatch
            .registry(PathEvent::NeedClose.as_ref().to_owned(), l)
    }

    pub fn append_write_event_handle<L>(&mut self, l: L)
    where
        L: Listener<PathEventInfo> + Send + Sync + 'static,
    {
        self.event_dispatch
            .registry(PathEvent::NeedWrite.as_ref().to_owned(), l)
    }

    pub fn append_open_event_handle<L>(&mut self, l: L)
    where
        L: Listener<PathEventInfo> + Send + Sync + 'static,
    {
        self.event_dispatch
            .registry(PathEvent::NeedOpen.as_ref().to_owned(), l)
    }

    fn dispatch_open_event(&mut self, pei: PathEventInfo) {
        self.event_dispatch
            .dispatch(PathEvent::NeedOpen.as_ref().to_string(), pei)
    }
    fn dispatch_write_event(&mut self, pei: PathEventInfo) {
        self.event_dispatch
            .dispatch(PathEvent::NeedWrite.as_ref().to_string(), pei)
    }
    fn dispatch_close_event(&mut self, pei: PathEventInfo) {
        self.event_dispatch
            .dispatch(PathEvent::NeedClose.as_ref().to_string(), pei)
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

    pub fn prepare_scan(&self) -> Result<Vec<PathEventInfo>> {
        println!("🔧 harvest auto_scanner start prepare_scanner!!!");
        let mut result = vec![];
        for entry in WalkDir::new(self.dir.clone()) {
            let entry = entry?;
            if !entry.path().is_file() {
                continue;
            }
            if let Some(pei) = Self::parse_path_to_pei(
                self.namespace.clone(),
                self.dir.clone(),
                entry.path().to_str().unwrap().to_owned(),
            ) {
                result.push(pei);
            }
        }
        Ok(result)
    }

    pub fn watch_start(&mut self) -> Result<()> {
        println!("🔧 harvest auto_scanner start");
        // Create a channel to receive the events.
        let (tx, rx) = channel();

        // Create a watcher object, delivering raw events.
        // The notification back-end is selected based on the platform.
        let mut watcher = raw_watcher(tx).unwrap();

        // Add a path to be watched. All files and directories at that path and
        // below will be monitored for changes.
        watcher.watch(&self.dir, RecursiveMode::Recursive)?;

        let cw = notify::Op::CREATE | notify::Op::WRITE;
        loop {
            match rx.recv() {
                Ok(RawEvent {
                    path: Some(path),
                    op: Ok(op),
                    cookie,
                }) => {
                    let path = path.to_str().unwrap().to_string();

                    match op {
                        notify::Op::CREATE => {
                            match Self::parse_path_to_pei(
                                self.namespace.to_owned(),
                                self.dir.clone(),
                                path.clone(),
                            ) {
                                Some(pei) => self.dispatch_open_event(pei),
                                _ => {}
                            }
                        }
                        notify::Op::WRITE => {
                            match Self::parse_path_to_pei(
                                self.namespace.to_owned(),
                                self.dir.clone(),
                                path.clone(),
                            ) {
                                Some(pei) => self.dispatch_write_event(pei),
                                _ => {}
                            }
                        }
                        notify::Op::REMOVE => {
                            match Self::parse_path_to_pei(
                                self.namespace.to_owned(),
                                self.dir.clone(),
                                path.clone(),
                            ) {
                                Some(pei) => self.dispatch_close_event(pei),
                                _ => {}
                            }
                        }
                        // cw => {
                        //     match Self::parse_path_to_pei(
                        //         self.namespace.to_owned(),
                        //         self.dir.clone(),
                        //         path.clone(),
                        //     ) {
                        //         Some(pei) => {
                        //             self.dispatch_open_event(pei.clone());
                        //             self.dispatch_write_event(pei)
                        //         }
                        //         _ => {}
                        //     }
                        // }
                        _ => {
                            println!("{:?} {:?} ({:?})", op, path, cookie);
                        }
                    }
                }
                Ok(event) => println!("scannner watch path: {:?} event: {:?}", &self.dir, event),
                Err(e) => {
                    println!("scannner watch path error: {:?}", e);
                    break;
                }
            }
        }

        // // Create a watcher object, delivering debounced events.
        // // The notification back-end is selected based on the platform.
        // let mut watcher = watcher(tx, Duration::from_millis(0))?;

        // // Add a path to be watched. All files and directories at that path and
        // // below will be monitored for changes.
        // if let Err(e) = watcher.watch(&self.dir, RecursiveMode::Recursive) {
        //     return Err(Box::new(e));
        // }
        // loop {
        //     match rx.recv() {
        //         Ok(event) => match event {
        //             DebouncedEvent::Create(path) => {
        //                 if let Some(path_str) = path.to_str() {
        //                     match Self::parse_path_to_pei(
        //                         self.namespace.to_owned(),
        //                         self.dir.clone(),
        //                         path_str.to_owned(),
        //                     ) {
        //                         Some(pei) => self
        //                             .event_dispatch
        //                             .dispatch(PathEvent::NeedOpen.as_ref().to_string(), pei),
        //                         _ => {}
        //                     }
        //                 }
        //             }
        //             DebouncedEvent::Write(path) => {
        //                 if let Some(path_str) = path.to_str() {
        //                     match Self::parse_path_to_pei(
        //                         self.namespace.to_owned(),
        //                         self.dir.clone(),
        //                         path_str.to_owned(),
        //                     ) {
        //                         Some(pei) => self
        //                             .event_dispatch
        //                             .dispatch(PathEvent::NeedWrite.as_ref().to_string(), pei),

        //                         _ => {}
        //                     }
        //                 }
        //             }
        //             DebouncedEvent::Remove(path) => {
        //                 if let Some(path_str) = path.to_str() {
        //                     match Self::parse_path_to_pei(
        //                         self.namespace.to_owned(),
        //                         self.dir.clone(),
        //                         path_str.to_owned(),
        //                     ) {
        //                         Some(pei) => self
        //                             .event_dispatch
        //                             .dispatch(PathEvent::NeedClose.as_ref().to_string(), pei),
        //                         _ => {}
        //                     }
        //                 }
        //             }
        //             DebouncedEvent::Error(e, path) => {
        //                 println!("watch event error: {:?} option: {:?}", e, path);
        //             }
        //             _ => {
        //                 println!("watch event {:?} not handle", event)
        //             }
        //         },
        //         Err(e) => {
        //             println!("watch error: {:?}", e);
        //             break;
        //         }
        //     }
        // }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{AutoScanner, GetDebug, PathEvent};
    use event::obj::Listener;
    use std::sync::mpsc::sync_channel as chan;

    #[test]
    fn it_works() {
        let (tx, rx) = chan::<String>(1);
        let mut auto_scanner = AutoScanner::new("".to_owned(), ".".to_owned());

        struct ListenerImpl;
        impl<T> Listener<T> for ListenerImpl
        where
            T: Clone + GetDebug,
        {
            fn handle(&self, t: T) {
                let x = t.get_debug();
            }
        }

        auto_scanner.append_close_event_handle(ListenerImpl);
    }

    #[test]
    fn event_it_works() {
        assert_eq!(PathEvent::NeedClose.as_ref(), "NeedClose");
        assert_eq!(PathEvent::NeedOpen.as_ref(), "NeedOpen");
        assert_eq!(PathEvent::NeedWrite.as_ref(), "NeedWrite");
    }
}
