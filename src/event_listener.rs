use db::{MemDatabase, GetPod};
use event::Listener;
use file::FileReaderWriter;
use log::{error as err, warn};
use scan::GetPathEventInfo;
use std::sync::{Arc, Mutex, RwLock};

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

pub(crate) struct DBAddEvent(pub Arc<RwLock<MemDatabase>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBAddEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        if let Some(pod) = t.get() {
            if !pod.upload {
                return;
            }

            let mut frw = match self.1.lock() {
                Ok(o) => o,
                Err(e) => {
                    err!("{}", e);
                    return;
                }
            };

            frw.open_event(pod.clone());
        }
    }
}

pub(crate) struct DBDeleteEvent(pub Arc<RwLock<MemDatabase>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBDeleteEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        let pod = match t.get() {
            None => return,
            Some(it) => it.clone(),
        };
        match self.1.lock() {
            Ok(mut o) => o.close_event(pod.uuid.clone()),
            Err(e) => {
                err!("DBDeleteEvent {}", e);
                return;
            }
        };
    }
}

pub(crate) struct DBUpdateEvent(pub Arc<RwLock<MemDatabase>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBUpdateEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        let pod = match t.get() {
            Some(pod) => pod.clone(),
            _ => return,
        };

        let mut frw = match self.1.lock() {
            Ok(f) => f,
            Err(e) => {
                warn!("DBUpdateEvent {}", e);
                return;
            }
        };

        if !pod.upload {
            frw.close_event(pod.uuid);
        } else {
            frw.open_event(pod);
        }
    }
}

pub(crate) struct ScannerWriteEvent(pub Arc<RwLock<MemDatabase>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerWriteEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let path = match t.get() {
            Some(it) => it.path.clone(),
            _ => return,
        };

        match self.1.lock() {
            Ok(mut o) => o.write_event(path),
            Err(e) => {
                err!("{}", e);
                return;
            }
        }
    }
}

pub(crate) struct ScannerOpenEvent(pub Arc<RwLock<MemDatabase>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerOpenEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let pod = match t.get() {
            Some(it) => it.to_pod(),
            _ => return,
        };

        match self.0.write() {
            Ok(mut o) => {
                // info!("event_listener open {}", pod.uuid.clone());
                o.put(pod)
            }
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        }
    }
}

pub(crate) struct ScannerCloseEvent(pub Arc<RwLock<MemDatabase>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerCloseEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let pei = t.get().unwrap();
        if let Ok(mut db) = self.0.write() {
            // info!("event_listener close {}", pei.path.to_owned());
            db.delete(pei.path.to_owned())
        }
    }
}
