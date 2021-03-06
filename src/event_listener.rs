use db::{Database, GetPod, State};
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
            let mut frw = match self.1.lock() {
                Ok(o) => o,
                Err(e) => {
                    eprintln!("{}", e);
                    return;
                }
            };

            frw.open_event((*pod).uuid.clone(), (*pod).offset, pod.output.clone());
        }
    }
}

pub(crate) struct DBDeleteEvent(pub Arc<RwLock<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBDeleteEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        if let Some(pod) = t.get() {
            // the event currently not impl
            println!("db delete pod {:?}", pod);
        }
    }
}

pub(crate) struct DBUpdateEvent(pub Arc<RwLock<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBUpdateEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        match t.get() {
            Some(pod) => match self.1.lock() {
                Ok(mut frw) => {
                    if (*pod).upload {
                        frw.open_event(
                            (&*pod.uuid).to_owned(),
                            (*pod).offset,
                            (&*pod.output).to_string(),
                        );

                        if let Ok(mut db) = self.0.write() {
                            let mut pod = pod.to_owned();
                            pod.state = State::Running;
                            db.update(pod.uuid.clone(), pod.clone());
                        }
                    } else {
                        if let Err(e) = frw.close_event((&*pod.uuid).to_owned()) {
                            eprintln!("{}", e)
                        }

                        if let Ok(mut db) = self.0.write() {
                            let mut pod = pod.to_owned();
                            pod.state = State::Stopped;
                            db.update(pod.uuid.clone(), pod.clone());
                        }
                    }
                }
                Err(e) => {
                    eprintln!("{}", e)
                }
            },

            None => {}
        }
    }
}

pub(crate) struct ScannerWriteEvent(pub Arc<RwLock<Database>>, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerWriteEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let pei = match t.get() {
            Some(it) => it,
            _ => return,
        };

        match self.1.lock() {
            Ok(mut o) => o.write_event((*pei).path.clone()),
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
        let pei = match t.get() {
            Some(it) => it,
            _ => return,
        };

        match self.0.write() {
            Ok(mut o) => o.put(pei.to_pod()),
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
