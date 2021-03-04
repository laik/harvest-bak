use db::{Database, GetPod};
use event::Listener;
use file::FileReaderWriter;
use scan::GetPathEventInfo;
use std::sync::{Arc, Mutex};

// TODO
// event list handle state
// scanner
//      close --> fileReaderWriter close
//      open  --> fileReaderWriter open
//      write --> fileReaderWriter write
// db
//      update --> upload false or true
//             |
//             true  --> open file and watch file && output
//             false --> close save offset
//

pub(crate) struct DBAddEvent(pub Arc<Mutex<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBAddEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        if let Some(pod) = t.get() {
            println!("db add pod {:?}", pod);
        }
    }
}

pub(crate) struct DBDeleteEvent(pub Arc<Mutex<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBDeleteEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        if let Some(pod) = t.get() {
            println!("db delete pod {:?}", pod);
        }
    }
}

pub(crate) struct DBUpdateEvent(pub Arc<Mutex<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBUpdateEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        if let Some(pod) = t.get() {
            println!("db update pod {:?}", pod);
        }
    }
}

pub(crate) struct ScannerWriteEvent(pub Arc<Mutex<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerWriteEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        println!("{:?}", t.get().unwrap())
    }
}

pub(crate) struct ScannerOpenEvent(pub Arc<Mutex<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerOpenEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let pei = t.get().unwrap();
        if let Ok(mut db) = self.0.lock() {
            if let Err(e) = db.put(pei.to_pod()) {
                eprintln!("{}", e)
            }
        }
    }
}

pub(crate) struct ScannerCloseEvent(pub Arc<Mutex<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerCloseEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let pei = t.get().unwrap();
        if let Ok(mut db) = self.0.lock() {
            if let Err(e) = db.delete(pei.path.to_owned()) {
                eprintln!("{}", e)
            }
        }
    }
}
