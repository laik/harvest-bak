use super::*;
use db::{AMemDatabase, GetPod};
use event::Listener;
use file::FileReaderWriter;
use log::{error as err, warn};
use scan::GetPathEventInfo;
use std::sync::{Arc, Mutex, RwLock};

pub(crate) struct DBAddEvent(pub AMemDatabase, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBAddEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        if let Some(pod) = t.get() {
            let mut need_open = false;
            if let Some(rule) = get_rule(pod.pod_name.clone()) {
                if rule.upload {
                    need_open = true;
                }
            };
            if !need_open {
                return;
            }

            let mut pod = pod.clone();
            pod.upload();

            let mut amdb = self.0.clone();
            amdb.apply(&pod);
        }
    }
}

pub(crate) struct DBDeleteEvent(pub AMemDatabase, pub Arc<Mutex<FileReaderWriter>>);
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

pub(crate) struct DBUpdateEvent(pub AMemDatabase, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for DBUpdateEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        let pod = match t.get() {
            Some(pod) => pod.clone(),
            _ => return,
        };

        if !pod.upload {
            return;
        }

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

pub(crate) struct ScannerWriteEvent(pub AMemDatabase, pub Arc<Mutex<FileReaderWriter>>);
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
                err!("ScannerWriteEvent lock frw error: {:?}", e);
                return;
            }
        }
    }
}

pub(crate) struct ScannerOpenEvent(pub AMemDatabase, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerOpenEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let pod = match t.get() {
            Some(it) => it.to_pod(),
            _ => return,
        };
        let mut amdb = self.0.clone();
        amdb.put(pod)
    }
}

pub(crate) struct ScannerCloseEvent(pub AMemDatabase, pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerCloseEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let pei = t.get().unwrap();
        let mut amdb = self.0.clone();
        amdb.delete(pei.path.to_owned())
    }
}
