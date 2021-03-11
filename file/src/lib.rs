#![feature(seek_stream_len)]
use db::{MemDatabase, Pod};
use log::{error as err, warn};
use output::OTS;
use serde_json::json;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, RwLock};
use std::thread;
use std::thread::JoinHandle;

pub enum SendFileEvent {
    Close,
    Other,
}

pub struct FileReaderWriter {
    handles: HashMap<String, (SyncSender<SendFileEvent>, JoinHandle<()>)>,
    database: Arc<RwLock<MemDatabase>>,
}

impl FileReaderWriter {
    pub fn new(database: Arc<RwLock<MemDatabase>>) -> Self {
        Self {
            handles: HashMap::new(),
            database,
        }
    }

    pub fn has(&self, path: &str) -> bool {
        if let Some(_) = self.handles.get(path) {
            return true;
        }
        false
    }

    pub fn close_event(&mut self, path: String) {
        if let Some((tx, jh)) = self.handles.get(&path) {
            if let Err(e) = tx.send(SendFileEvent::Close) {
                err!(
                    "frw send close event to path FileReaderWriter {:?} error: {:?}",
                    &path,
                    e
                );
                let mut _jh = &*jh;
            }
            self.handles.remove(&path);
        };
    }

    fn open(&mut self, pod: Pod) {
        let thread_path = pod.uuid.clone();
        let database = self.database.clone();

        let (tx, rx) = sync_channel::<SendFileEvent>(1);
        let mut offset = pod.offset;

        let mut file = match File::open(thread_path.clone()) {
            Ok(file) => file,
            Err(e) => {
                err!("frw open file {:?} error: {:?}", pod.uuid, e);
                return;
            }
        };
        if let Err(e) = file.seek(SeekFrom::Current(offset)) {
            err!("frw open event seek failed, error: {}", e);
            return;
        }

        let file_size = file.stream_len().unwrap();
        let mut br = BufReader::new(file);
        let mut bf = String::new();
        let outputs = OTS.clone();

        loop {
            let incr_offset = br.read_line(&mut bf).unwrap();
            if let Ok(mut ot) = outputs.lock() {
                ot.output(pod.output.clone(), &encode_message(&pod, bf.as_str()))
            }
            offset += incr_offset as i64;
            bf.clear();
            if offset >= file_size as i64 {
                break;
            }
        }

        let thread_pod = pod.clone();
        let jh = thread::spawn(move || loop {
            for item in rx.recv() {
                match item {
                    SendFileEvent::Close => {
                        break;
                    }
                    _ => {}
                }
                let incr_offset = br.read_line(&mut bf).unwrap();
                if let Ok(mut ot) = outputs.lock() {
                    ot.output(
                        thread_pod.output.clone(),
                        &encode_message(&thread_pod, bf.as_str()),
                    )
                }

                if let Ok(mut database) = database.try_write() {
                    database.incr_offset_by_uuid(thread_path.clone(), incr_offset as i64);
                } else {
                    break;
                }

                bf.clear();
            }
        });
        self.handles.insert(pod.uuid, (tx, jh));
    }

    pub fn open_event(&mut self, pod: Pod) {
        if self.handles.contains_key(&pod.uuid) {
            return;
        }
        self.open(pod);
    }

    pub fn write_event(&mut self, path: String) {
        let mut pod = None;
        if !self.handles.contains_key(&path) {
            match self.database.read() {
                Ok(db) => {
                    match db.get(path.clone()) {
                        Some(it) => {
                            if !it.upload {
                                return;
                            }
                            pod = Some(it.clone());
                        }
                        None => {
                            return;
                        }
                    };
                }
                Err(e) => {
                    err!("frw write event read db error {:?}", e)
                }
            }
            self.open(pod.unwrap());
        }

        let handle = match self.handles.get(&path) {
            Some(it) => it,
            _ => {
                warn!("frw not found handle {}", path);
                return;
            }
        };

        if let Err(e) = handle.0.send(SendFileEvent::Other) {
            err!("frw send write event error: {}, path: {}", e, path)
        }
    }
}

fn encode_message<'a>(pod: &'a Pod, message: &'a str) -> String {
    if message.len() == 0 {
        return "".to_owned();
    }
    json!({"custom":
            {
            "nodeId":pod.pod_name,
            "container":pod.container_name,
            "serviceName":pod.pod_name,
            "ips":["172.0.0.1"],
            "version":"v1.0.0"
            },
        "message":message}
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use crate::FileReaderWriter;
    use db::{new_arc_database, MemDatabase, Pod};

    #[test]
    fn it_works() {
        let mut input = FileReaderWriter::new(new_arc_database(MemDatabase::default()));
        input.open_event(Pod::default());
    }
}
