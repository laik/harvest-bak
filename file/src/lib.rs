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

pub struct Input(HashMap<String, (SyncSender<()>, JoinHandle<()>)>);

impl Input {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn open_event<T>(
        &mut self,
        path: String,
        offset: i64,
        db: Arc<Mutex<Database>>,
        output: Arc<Mutex<T>>,
    ) -> Result<()>
    where
        T: IOutput,
    {
        if self.0.contains_key(&path) {
            return Ok(());
        }

        match |path: String,
               offset: i64,
               db: Arc<Mutex<Database>>,
               output: Arc<Mutex<T>>|
         -> Result<(SyncSender<()>, JoinHandle<()>)> {
            let (tx, rx) = sync_channel(1);
            let join_handle = thread::spawn(move || match File::open(path.clone()) {
                Ok(mut f) => {
                    if let Err(e) = f.seek(SeekFrom::Current(offset)) {
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
        }(path.clone(), offset, db.clone(), output)
        {
            Ok(res) => {
                self.0.insert(path.clone(), res);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    pub fn write_event(&mut self, path: String) -> Result<()> {
        if !self.0.contains_key(&path) {
            return Ok(());
        }
        match self.0.get(&path).unwrap().0.send(()) {
            Ok(_) => Ok(()),
            Err(e) => Err(Box::new(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Input;
    use db::{new_sync_database, Database, Pod};
    use event::EventHandler;
    use output::{new_sync_output, FakeOutput};

    #[test]
    fn it_works() {
        let mut input = Input::new();
        let output = new_sync_output(FakeOutput);
        let db = new_sync_database(Database::new(EventHandler::<Pod>::new()));
        if let Err(e) = input.open_event("./lib.rs".to_owned(), 0, db, output) {
            assert_eq!(format!("{}", e), "No such file or directory (os error 2)");
        }
    }
}
