use common::Result;
use db::Database;
use output::{sync_via_output, IOutput};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread;
use std::thread::JoinHandle;

pub struct FileReaderWriter {
    handles: HashMap<String, (SyncSender<()>, JoinHandle<()>)>,
    db: Arc<Mutex<Database>>,
    outputs: HashMap<String, Arc<Mutex<dyn IOutput>>>,
}

impl FileReaderWriter {
    pub fn new(db: Arc<Mutex<Database>>) -> Self {
        Self {
            handles: HashMap::new(),
            db,
            outputs: HashMap::new(),
        }
    }

    pub fn set_output(&mut self, _type: String, output: Arc<Mutex<dyn IOutput>>) {
        self.outputs.insert(_type, output);
    }

    pub fn has(&self, path: &str) -> bool {
        if let Some(_) = self.handles.get(path) {
            return true;
        }
        false
    }

    pub fn close_event(&mut self, path: String) -> Result<()> {
        match self.handles.get(&path) {
            Some(_item) => {
                self.handles.remove(&path);
            }
            None => {}
        }
        Ok(())
    }

    pub fn open_event(&mut self, path: String, output_type: &str) -> Result<()> {
        if self.handles.contains_key(&path) {
            return Ok(());
        }

        if let Some(_output) = self.outputs.get(output_type) {
            match Self::_open(path.clone(), self.db.clone(), _output.clone()) {
                Ok(res) => {
                    self.handles.insert(path.clone(), res);
                    Ok(())
                }
                Err(e) => Err(e),
            }
        } else {
            eprintln!("not found output type: {}", output_type);
            Ok(())
        }
    }

    pub fn write_event(&mut self, path: String) -> Result<()> {
        if !self.handles.contains_key(&path) {
            return Ok(());
        }
        match self.handles.get(&path).unwrap().0.send(()) {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }

    fn _open(
        path: String,
        db: Arc<Mutex<Database>>,
        output: Arc<Mutex<dyn IOutput>>,
    ) -> Result<(SyncSender<()>, JoinHandle<()>)> {
        let mut offset = 0;

        if let Ok(db) = db.lock() {
            if let Some(pod) = db.get(path.clone()) {
                offset = pod.offset;
            }
        }

        let (tx, rx) = sync_channel(1);
        let join_handle = thread::spawn(move || match File::open(path.clone()) {
            Ok(mut f) => {
                if let Err(e) = f.seek(SeekFrom::Current(offset as i64)) {
                    eprintln!("{}", e);
                    return;
                }
                let mut buf_r = BufReader::new(f);
                let mut buf = String::new();
                for _ in rx.recv() {
                    match db.lock() {
                        Ok(mut _db) => {
                            _db.incr_offset_by_uuid(
                                path.clone(),
                                buf_r.read_line(&mut buf).unwrap(),
                            );

                            if let Err(e) = sync_via_output(buf.as_str(), output.clone()) {
                                eprintln!("{}", e);
                            }
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                    buf.clear();
                }
            }
            Err(e) => eprintln!("{}", e),
        });

        Ok((tx, join_handle))
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use crate::FileReaderWriter;
    use db::{new_sync_database, Database, Pod};
    use event::obj::Dispatch;
    use output::{FakeOutput, Output};

    #[test]
    fn it_works() {
        let fake_output = Arc::new(Mutex::new(Output::new(FakeOutput)));
        let mut input =
            FileReaderWriter::new(new_sync_database(Database::new(Dispatch::<Pod>::new())));

        input.set_output("fake_output".to_owned(), fake_output);
        if let Err(e) = input.open_event("./lib.rs".to_owned(), &"fake_output") {
            assert_eq!(format!("{}", e), "No such file or directory (os error 2)");
        }
    }
}
