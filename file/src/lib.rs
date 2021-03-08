#![feature(seek_stream_len)]
use db::Database;
use output::{via_output, FakeOutput, IOutput, Output};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::thread::JoinHandle;

pub enum SendFileEvent {
    Close,
    Other,
}

pub struct FileReaderWriter {
    handles: HashMap<String, (SyncSender<SendFileEvent>, JoinHandle<()>)>,
    database: Arc<RwLock<Database>>,
    outputs: HashMap<String, Arc<Mutex<dyn IOutput>>>,
}

impl FileReaderWriter {
    pub fn new(database: Arc<RwLock<Database>>) -> Self {
        Self {
            handles: HashMap::new(),
            database,
            outputs: HashMap::new(),
        }
    }

    pub fn set_output(&mut self, output_name: String, output: Arc<Mutex<dyn IOutput>>) {
        self.outputs.insert(output_name, output);
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
                eprintln!(
                    "send close event to path FileReaderWriter {:?} error: {:?}",
                    &path, e
                );
                let mut _jh = &*jh;
                // _jh.join();
            }
            self.handles.remove(&path);
        };
    }

    pub fn open_event(&mut self, path: String, offset: i64, output: String) {
        if self.handles.contains_key(&path) {
            return;
        }
        let thread_path = path.clone();
        let database = self.database.clone();

        let (tx, rx) = sync_channel::<SendFileEvent>(1);
        let mut offset = offset;
        let jh = thread::spawn(move || {
            if let Ok(mut file) = File::open(thread_path.clone()) {
                let size = file.stream_len().unwrap();
                // println!("file {:?} length {:?}", thread_path.clone(), size);
                // skip to current file end
                if size as i64 > offset {
                    offset = size as i64;
                }
                if let Err(e) = file.seek(SeekFrom::Current(offset)) {
                    eprintln!("open event seek failed, error: {:?}", e);
                    return;
                }
                let mut br = BufReader::new(file);
                let mut bf = String::new();
                let output = &mut Output::new(FakeOutput);

                loop {
                    for item in rx.recv() {
                        match item {
                            SendFileEvent::Close => {
                                break;
                            }
                            _ => {}
                        }
                        let offset = br.read_line(&mut bf).unwrap();

                        if let Err(e) = via_output(bf.as_str(), output) {
                            eprintln!("{}", e);
                        }
                        if let Ok(mut database) = database.try_write() {
                            database.incr_offset_by_uuid(thread_path.clone(), offset as i64);
                        } else {
                            break;
                        }

                        bf.clear();
                    }
                }
            }
        });
        self.handles.insert(path.clone(), (tx, jh));
    }

    pub fn write_event(&mut self, path: String) {
        if !self.handles.contains_key(&path) {
            return;
        }

        let handle = match self.handles.get(&path) {
            Some(it) => it,
            _ => return,
        };

        match handle.0.send(SendFileEvent::Other) {
            Err(e) => eprintln!(
                "FileReaderWriter send write event error: {:?},path: {:?}",
                e,
                path.clone()
            ),
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::FileReaderWriter;
    use db::{new_arc_database, Database};
    use output::{FakeOutput, Output};
    use std::sync::{Arc, Mutex};

    #[test]
    fn it_works() {
        let fake_output = Arc::new(Mutex::new(Output::new(FakeOutput)));
        let mut input = FileReaderWriter::new(new_arc_database(Database::default()));

        input.set_output("fake_output".to_owned(), fake_output);
        input.open_event("./lib.rs".to_owned(), 0, "fake_output".to_owned());
    }
}
