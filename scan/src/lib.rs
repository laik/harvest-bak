use common::Result;
use db::Database;
use event::EventHandler;
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::{
    sync::{mpsc::channel, Arc, Mutex},
    time::Duration,
};
use strum::AsRefStr;

#[derive(Debug, AsRefStr, Clone)]
pub enum PathEvent {
    #[strum(serialize = "NeedOpen")]
    NeedOpen,
    #[strum(serialize = "NeedClose")]
    NeedClose,
    #[strum(serialize = "NeedWrite")]
    NeedWrite,
}

#[derive(Debug, Clone)]
pub struct PathEventInfo {
    namespace: String,
    pod: String,
    container: String,
    path: String,
}

unsafe impl Sync for PathEventInfo {}
unsafe impl Send for PathEventInfo {}

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

    //TODO
    fn parse_to_event_info(namespace: String, path: String) -> Option<PathEventInfo> {
        Some(PathEventInfo {
            namespace,
            pod: "".to_string(),
            container: "".to_string(),
            path,
        })
    }

    // 首先list符合namespace的目录，然后watch目录的变更
    pub fn start(
        &self,
        path_event_handler: EventHandler<(PathEventInfo, Arc<Mutex<Database>>)>,
    ) -> Result<()> {
        println!("auto_scanner start");
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

        loop {
            match rx.recv() {
                Ok(event) => match event {
                    DebouncedEvent::Create(path) => {
                        if let Some(path_str) = path.to_str() {
                            match Self::parse_to_event_info(
                                self.namespace.to_owned(),
                                path_str.to_owned(),
                            ) {
                                Some(pei) => {
                                    path_event_handler.event(
                                        PathEvent::NeedOpen.as_ref().to_owned(),
                                        (pei, self.db.clone()),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    DebouncedEvent::Write(path) => {
                        if let Some(path_str) = path.to_str() {
                            match Self::parse_to_event_info(
                                self.namespace.to_owned(),
                                path_str.to_owned(),
                            ) {
                                Some(pei) => {
                                    path_event_handler.event(
                                        PathEvent::NeedWrite.as_ref().to_owned(),
                                        (pei, self.db.clone()),
                                    );
                                }
                                _ => {}
                            }
                        }
                    }
                    DebouncedEvent::Remove(path) => {
                        if let Some(path_str) = path.to_str() {
                            match Self::parse_to_event_info(
                                self.namespace.to_owned(),
                                path_str.to_owned(),
                            ) {
                                Some(pei) => {
                                    path_event_handler.event(
                                        PathEvent::NeedClose.as_ref().to_owned(),
                                        (pei, self.db.clone()),
                                    );
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

#[warn(dead_code)]
struct ParsePodForPath {
    pod: String,
    namespace: String,
    container: String,
    logfiles: Vec<String>,
}
// the path eg:
// /var/log/pod
//db_mysql-apollo-slave-0_49d0b6e1-9980-4f7b-b1eb-3eab3e753b48
// └── mysql
//     ├── 4.log -> /data/docker/containers/1707c92da3df11616bd8eb15bf1c8e60105e5276b62acba3c0aa12e3d0f03df5/1707c92da3df11616bd8eb15bf1c8e60105e5276b62acba3c0aa12e3d0f03df5-json.log
//     └── 5.log -> /data/docker/containers/5b3c5c7cd28f42a3e5320c8f0e64988de2ddeb6f87223c833045f1e0fcf74528/5b3c5c7cd28f42a3e5320c8f0e64988de2ddeb6f87223c833045f1e0fcf74528-json.log
// expect:
// ParsePodForPath{
//  pod: mysql-apollo-slave-0
//  namespace: db
//  container: mysql
//  logfiles: [4.log,5.log]
// }
// /var/log/pods/jingxiao-dev_yame-pds-0-b-0_cc3c4800-f171-4dca-80ef-815d50d54e8e/yame-pds/153.log
fn parse_pod_for_path(dir: String, path: String) -> Option<ParsePodForPath> {
    if !path.ends_with(".log") {
        return None;
    }

    Some(ParsePodForPath {
        pod: "".to_owned(),
        namespace: "".to_owned(),
        container: "".to_owned(),
        logfiles: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    use crate::{AutoScanner, PathEvent, PathEventInfo};
    use db::Database;
    use event::EventHandler;

    #[test]
    fn it_works() {
        trait Handle: Sync + Send {
            fn handle(&self, pei: PathEventInfo, db: Arc<Mutex<Database>>) {
                println!("mock listener {:?}", pei);
            }
        }

        let maps = Arc::new(HashMap::<String, Arc<dyn Handle>>::new());

        let datanase = Arc::new(Mutex::new(Database::new(EventHandler::new())));
        let auto_scanner = AutoScanner::new("default".to_owned(), "".to_owned(), datanase.clone());

        // new a event_handler registry mock action events handle
        let mut path_event_handler = EventHandler::<(PathEventInfo, Arc<Mutex<Database>>)>::new();

        path_event_handler.registry(
            PathEvent::NeedClose.as_ref().to_owned(),
            move |(pei, db)| {
                if let Some(h) = maps.get(&pei.path) {
                    h.handle(pei, db);
                }
            },
        );

        // start but this was occurred "No path was found."
        if let Err(e) = auto_scanner.start(path_event_handler) {
            assert_eq!("No path was found.", e.to_string());
        }
    }

    #[test]
    fn event_it_works() {
        assert_eq!(PathEvent::NeedClose.as_ref(), "NeedClose");
        assert_eq!(PathEvent::NeedOpen.as_ref(), "NeedOpen");
        assert_eq!(PathEvent::NeedWrite.as_ref(), "NeedWrite");
    }
}
