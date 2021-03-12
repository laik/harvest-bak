#[warn(mutable_borrow_reservation_conflict)]
#[macro_use] extern crate lazy_static;
mod database;

use std::sync::{Arc, RwLock};

pub(crate) use database::MemDatabase;
pub use database::{Event, GetPod, Pod, State};
use event::Listener;

lazy_static! {
    pub static ref AMDB: AMemDatabase = {
        let m = AMemDatabase::new();
        m
    };
}

#[derive(Clone)]
pub struct AMemDatabase(Arc<RwLock<MemDatabase>>);

impl AMemDatabase {
    pub fn new() -> Self {
        Self(Arc::new(RwLock::new(MemDatabase::new())))
    }

    pub fn append_add_event<L>(&mut self, l: L)
    where
        L: Listener<Pod> + Send + Sync + 'static,
    {
        match self.0.write() {
            Ok(mut mdb) => mdb.append_add_event(l),
            Err(e) => {
                eprintln!("{:?}", e)
            }
        }
    }

    pub fn get(&self, uuid: &str) -> Option<Pod> {
        if let Ok(mdb) = self.0.read() {
            if let Some(pod) = mdb.pods.get(uuid) {
                return Some(pod.to_owned());
            }
        }
        None
    }

    pub fn append_delete_event<L>(&mut self, l: L)
    where
        L: Listener<Pod> + Send + Sync + 'static,
    {
        match self.0.write() {
            Ok(mut mdb) => mdb.append_delete_event(l),
            Err(e) => {
                eprintln!("{:?}", e)
            }
        }
    }

    pub fn append_update_event<L>(&mut self, l: L)
    where
        L: Listener<Pod> + Send + Sync + 'static,
    {
        match self.0.write() {
            Ok(mut mdb) => mdb.append_update_event(l),
            Err(e) => {
                eprintln!("{:?}", e)
            }
        }
    }

    pub fn all_to_json(&self) -> String {
        match self.0.read() {
            Ok(mdb) => mdb.all().to_json(),
            Err(e) => {
                eprintln!("{:?}", e);
                "".into()
            }
        }
    }

    pub fn get_slice_by_ns_pod(&self, ns: String, pod: String) -> Vec<(String, Pod)> {
        match self.0.read() {
            Ok(mdb) => mdb.get_slice_by_ns_pod(ns, pod),
            Err(e) => {
                eprintln!("{:?}", e);
                vec![]
            }
        }
    }

    pub fn apply(&mut self, pod: &Pod) {
        match self.0.write() {
            Ok(mut mdb) => mdb.apply(pod),
            Err(e) => {
                eprintln!("{:?}", e);
            }
        }
    }

    pub fn delete_by_ns_pod(&mut self, ns: String, pod_name: String) {
        match self.0.write() {
            Ok(mut mdb) => mdb.delete_by_ns_pod(ns, pod_name),
            Err(e) => {
                eprintln!("{:?}", e);
            }
        }
    }

    pub fn delete(&mut self, uuid: String) {
        match self.0.write() {
            Ok(mut mdb) => mdb.delete(uuid),
            Err(e) => {
                eprintln!("{:?}", e);
            }
        }
    }

    pub fn stop_upload_pod(&mut self, ns: String, pod_name: String) {
        match self.0.write() {
            Ok(mut mdb) => mdb.stop_upload_pod(ns, pod_name),
            Err(e) => {
                eprintln!("{:?}", e);
            }
        }
    }

    pub fn start_upload_pod(&mut self, ns: String, pod_name: String) {
        match self.0.write() {
            Ok(mut mdb) => mdb.start_upload_pod(ns, pod_name),
            Err(e) => {
                eprintln!("{:?}", e);
            }
        }
    }

    pub fn incr_offset_by_uuid(&mut self, uuid: String, incr_size: i64) {
        match self.0.write() {
            Ok(mut mdb) => mdb.incr_offset_by_uuid(uuid, incr_size),
            Err(e) => {
                eprintln!("{:?}", e)
            }
        }
    }

    pub fn put(&mut self, pod: Pod) {
        match self.0.write() {
            Ok(mut mdb) => mdb.put(pod),
            Err(e) => {
                eprintln!("{:?}", e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossbeam_channel::unbounded;
    use std::vec;
    use std::{thread, time};

    #[test]
    fn thread_safe() {
        let mut amdb = AMemDatabase::new();
        amdb.put(Pod {
            pod_name: "a".into(),
            ..Default::default()
        });
        amdb.put(Pod {
            pod_name: "b".into(),
            ..Default::default()
        });
        amdb.put(Pod {
            pod_name: "c".into(),
            ..Default::default()
        });
        let (t, r) = unbounded::<()>();

        let mut joins = vec![];
        let mut x = amdb.clone();
        let r1 = r.clone();
        let r2 = r.clone();
        let r3 = r.clone();
        let r4 = r.clone();

        joins.push(thread::spawn(move || {
            let mut x1 = x.clone();
            thread::spawn(move || loop {
                match r1.recv() {
                    Ok(_) => {
                        return;
                    }
                    Err(e) => {}
                };
                x1.incr_offset_by_uuid("a".into(), 1);
            });
            loop {
                match r2.recv() {
                    Ok(_) => {
                        return;
                    }
                    Err(e) => {}
                };
                x.incr_offset_by_uuid("a".into(), 1);
            }
        }));
        let mut x = amdb.clone();
        joins.push(thread::spawn(move || loop {
            match r3.recv() {
                Ok(_) => {
                    return;
                }
                Err(e) => {}
            };
            x.incr_offset_by_uuid("b".into(), 1);
        }));

        let mut x = amdb.clone();
        joins.push(thread::spawn(move || loop {
            match r4.recv() {
                Ok(_) => {
                    return;
                }
                Err(e) => {}
            };
            x.incr_offset_by_uuid("c".into(), 1);
        }));

        let ten = time::Duration::from_secs(2);
        // let now = time::Instant::now();
        thread::sleep(ten);

        for _ in 0..4 {
            t.send(()).unwrap();
        }

        for th in joins {
            th.join().unwrap();
        }
    }
}
