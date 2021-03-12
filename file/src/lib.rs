#![feature(seek_stream_len)]
extern crate crossbeam_channel;
use crossbeam_channel::{unbounded as async_channel, Sender};
use db::{AMemDatabase, Pod};
use log::{error as err, warn};
use output::OTS;
use serde_json::json;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use threadpool::ThreadPool;

pub enum SendFileEvent {
    Close,
    Other,
}

pub struct FileReaderWriter {
    threadpool: ThreadPool,
    handles: HashMap<String, Sender<SendFileEvent>>,
    amdb: AMemDatabase,
}

impl FileReaderWriter {
    pub fn new(amdb: AMemDatabase, num_workers: usize) -> Self {
        Self {
            threadpool: ThreadPool::new(num_workers),
            handles: HashMap::new(),
            amdb,
        }
    }

    pub fn has(&self, path: &str) -> bool {
        if let Some(_) = self.handles.get(path) {
            return true;
        }
        false
    }

    pub fn close_event(&mut self, path: String) {
        if let Some(tx) = self.handles.get(&path) {
            if let Err(e) = tx.send(SendFileEvent::Close) {
                err!(
                    "frw send close event to path FileReaderWriter {:?} error: {:?}",
                    &path,
                    e
                );
            }
            self.handles.remove(&path);
        };
    }

    fn open(&mut self, pod: Pod) {
        let thread_path = pod.uuid.clone();
        let mut amdb = self.amdb.clone();

        let (tx, rx) = async_channel::<SendFileEvent>();
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

            amdb.incr_offset_by_uuid(thread_path.clone(), incr_offset as i64);
        }

        let thread_pod = pod.clone();
        self.threadpool.execute(move || loop {
            match rx.recv() {
                Ok(item) => match item {
                    SendFileEvent::Close => {
                        break;
                    }
                    _ => {}
                },
                Err(e) => {
                    err!("{}", e)
                }
            }
            let incr_offset = br.read_line(&mut bf).unwrap();
            if let Ok(mut ot) = outputs.lock() {
                ot.output(
                    thread_pod.output.clone(),
                    &encode_message(&thread_pod, bf.as_str()),
                )
            }

            amdb.incr_offset_by_uuid(thread_path.clone(), incr_offset as i64);
            bf.clear();
        });
        self.handles.insert(pod.uuid, tx);
    }

    pub fn open_event(&mut self, pod: Pod) {
        if self.handles.contains_key(&pod.uuid) {
            return;
        }
        self.open(pod);
    }

    pub fn write_event(&mut self, path: String) {
        if !self.handles.contains_key(&path) {
            match self.amdb.get(&*path) {
                Some(it) => {
                    if !it.upload {
                        return;
                    }
                    self.open(it);
                }
                None => {
                    return;
                }
            };
        }

        let handle = match self.handles.get(&path) {
            Some(it) => it,
            _ => {
                warn!("frw not found handle {}", path);
                return;
            }
        };

        if let Err(e) = handle.send(SendFileEvent::Other) {
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
            "ips":pod.ips,
            "version":"v1.0.0"
            },
        "message":message}
    )
    .to_string()
}

#[cfg(test)]
mod tests {
    use crate::FileReaderWriter;
    use db::{AMemDatabase, Pod};

    #[test]
    fn it_works() {
        let mut input = FileReaderWriter::new(AMemDatabase::new(), 10);
        input.open_event(Pod::default());
    }
}
