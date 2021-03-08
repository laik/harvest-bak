use db::{Database, GetPod};
use event::Listener;
use file::FileReaderWriter;
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

pub(crate) struct DBAddEvent(pub Arc<RwLock<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBAddEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        if let Some(pod) = t.get() {
            println!("db event add {:?}", pod);
        }
    }
}

pub(crate) struct DBDeleteEvent(pub Arc<RwLock<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBDeleteEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        let pod = match t.get() {
            None => return,
            Some(it) => it.clone(),
        };
        match self.1.try_lock() {
            Ok(mut o) => o.close_event(pod.uuid.clone()),
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };
    }
}

pub(crate) struct DBUpdateEvent(pub Arc<RwLock<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBUpdateEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        let mut pod = match t.get() {
            Some(pod) => pod.clone(),
            _ => return,
        };

        let mut frw = match self.1.lock() {
            Ok(f) => f,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };

        match pod.upload {
            true => {
                frw.open_event(
                    (&*pod.uuid).to_string(),
                    pod.offset,
                    (&*pod.output).to_string(),
                );
                pod.set_running()
            }
            false => {
                frw.close_event((&*pod.uuid).to_owned());
                pod.set_stopped()
            }
        };

        if let Ok(mut db) = self.0.write() {
            db.update(pod.uuid.clone(), pod.clone())
        }
    }
}

pub(crate) struct ScannerWriteEvent(pub Arc<RwLock<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerWriteEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let path = match t.get() {
            Some(it) => it.path.clone(),
            _ => return,
        };

        println!("watch scanner write event, path {:?}", path);

        match self.1.lock() {
            Ok(mut o) => {
                o.write_event(path)
            }
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        }
    }
}

pub(crate) struct ScannerOpenEvent(pub Arc<RwLock<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerOpenEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let pod = match t.get() {
            Some(it) => it.to_pod(),
            _ => return,
        };

        let mut frw = match self.1.lock() {
            Ok(o) => o,
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        };

        frw.open_event(pod.uuid.clone(), pod.offset, pod.output.clone());

        match self.0.write() {
            Ok(mut o) => {
                println!("watch scanner open event add pod {:?}", pod);
                o.put(pod)
            }
            Err(e) => {
                eprintln!("{}", e);
                return;
            }
        }
    }
}

pub(crate) struct ScannerCloseEvent(pub Arc<RwLock<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerCloseEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let pei = t.get().unwrap();
        if let Ok(mut db) = self.0.write() {
            db.delete(pei.path.to_owned())
        }
    }
}
