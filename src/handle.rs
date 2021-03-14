use event::Listener;
use file::FileReaderWriter;
use log::error as err;
use scan::GetPathEventInfo;
use std::sync::{Arc, Mutex};


pub(crate) struct ScannerOpenEvent(pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerOpenEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        match self.0.lock() {
            Ok(mut frw) => frw.open_event(&mut t.get().to_pod()),
            Err(e) => {
                err!("{:?}", e)
            }
        }
    }
}

pub(crate) struct ScannerWriteEvent(pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerWriteEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        match self.0.lock() {
            Ok(mut frw) => frw.write_event(&mut t.get().to_pod()),
            Err(e) => {
                err!("{:?}", e);
            }
        }
    }
}

pub(crate) struct ScannerCloseEvent(pub Arc<Mutex<FileReaderWriter>>);
impl<T> Listener<T> for ScannerCloseEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        match self.0.lock() {
            Ok(mut frw) => frw.close_event(&t.get().to_pod()),
            Err(e) => {
                err!("{:?}", e)
            }
        }
    }
}
