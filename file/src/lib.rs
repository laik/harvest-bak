use common::Result;
use db::Database;
use output::{sync_via_output, via_output, FakeOutput, IOutput, Output};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::sync::mpsc::{sync_channel, SyncSender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;
use std::thread::JoinHandle;

pub struct FileReaderWriter {
    handles: HashMap<String, (SyncSender<()>, JoinHandle<()>)>,
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

    pub fn close_event(&mut self, path: String) -> Result<()> {
        match self.handles.get(&path) {
            Some(_item) => {
                self.handles.remove(&path);
            }
            None => {}
        }
        Ok(())
    }

    pub fn open_event(&mut self, path: String, offset: i64, output: String) -> Result<()> {
        if self.handles.contains_key(&path) {
            return Ok(());
        }

        let tpath = path.clone();
        let database = self.database.clone();

        let (tx, rx) = sync_channel(1);
        let jh = thread::spawn(move || {
            if let Ok(mut file) = File::open(tpath.clone()) {
                if let Err(e) = file.seek(SeekFrom::Current(offset)) {
                    eprintln!("{}", e);
                    return;
                }
                let mut br = BufReader::new(file);
                let mut bf = String::new();
                for _ in rx.recv() {
                    match br.read_line(&mut bf) {
                        Ok(offset) => {
                            let output = &mut Output::new(FakeOutput);
                            if let Err(e) = via_output(bf.as_str(), output) {
                                eprintln!("{}", e);
                            }
                            database
                                .try_write()
                                .unwrap()
                                .incr_offset_by_uuid(tpath.clone(), offset as i64);
                        }
                        Err(e) => eprintln!("{}", e),
                    }
                    bf.clear();
                }
            }
        });
        self.handles.insert(path.clone(), (tx, jh));

        Ok(())
    }

    pub fn write_event(&mut self, path: String) -> Result<()> {
        if !self.handles.contains_key(&path) {
            return Ok(());
        }

        let handle = match self.handles.get(&path) {
            Some(it) => it,
            _ => return Ok(()),
        };

        match handle.0.send(()) {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
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
        if let Err(e) = input.open_event("./lib.rs".to_owned(), 0, "fake_output".to_owned()) {
            assert_eq!(format!("{}", e), "No such file or directory (os error 2)");
        }
    }
}
