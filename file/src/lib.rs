#![feature(seek_stream_len)]
use db::Database;
use log::error as err;
use output::OTS;
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
    database: Arc<RwLock<Database>>,
}

impl FileReaderWriter {
    pub fn new(database: Arc<RwLock<Database>>) -> Self {
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
                    "send close event to path FileReaderWriter {:?} error: {:?}",
                    &path,
                    e
                );
                let mut _jh = &*jh;
            }
            self.handles.remove(&path);
        };
    }

    fn open(&mut self, path: String, offset: i64, _output: String) {
        let thread_path = path.clone();
        let database = self.database.clone();

        let (tx, rx) = sync_channel::<SendFileEvent>(1);
        let mut offset = offset;

        let mut file = match File::open(thread_path.clone()) {
            Ok(file) => file,
            Err(e) => {
                err!("open file {:?} error: {:?}", path, e);
                return;
            }
        };
        if let Err(e) = file.seek(SeekFrom::Current(offset)) {
            err!("open event seek failed, error: {}", e);
            return;
        }

        let file_size = file.stream_len().unwrap();
        let mut br = BufReader::new(file);
        let mut bf = String::new();
        let outputs = OTS.clone();

        loop {
            let incr_offset = br.read_line(&mut bf).unwrap();
            if let Ok(mut ot) = outputs.lock() {
                ot.output(_output.clone(), bf.as_str())
            }
            offset += incr_offset as i64;
            bf.clear();
            if offset >= file_size as i64 {
                break;
            }
        }

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
                    ot.output(_output.clone(), bf.as_str())
                }

                if let Ok(mut database) = database.try_write() {
                    database.incr_offset_by_uuid(thread_path.clone(), incr_offset as i64);
                } else {
                    break;
                }

                bf.clear();
            }
        });
        self.handles.insert(path.clone(), (tx, jh));
    }

    pub fn open_event(&mut self, path: String, offset: i64, _output: String) {
        if self.handles.contains_key(&path) {
            return;
        }
        self.open(path, offset, _output)
    }

    pub fn write_event(&mut self, path: String) {
        if !self.handles.contains_key(&path) {
            self.open(path.clone(), 0, "".to_owned())
        }

        let handle = match self.handles.get(&path) {
            Some(it) => it,
            _ => return,
        };

        if let Err(e) = handle.0.send(SendFileEvent::Other) {
            err!("send write event error: {},path: {}", e, path)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::FileReaderWriter;
    use db::{new_arc_database, Database};

    #[test]
    fn it_works() {
        let mut input = FileReaderWriter::new(new_arc_database(Database::default()));
        input.open_event("./lib.rs".to_owned(), 0, "fake_output".to_owned());
    }
}
