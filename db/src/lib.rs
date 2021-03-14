#[warn(mutable_borrow_reservation_conflict)]
#[macro_use]
extern crate lazy_static;
mod database;

mod pod;
use database::Message;
pub use pod::{GetPod, Pod, PodList, PodListMarshaller, State};

pub use database::Event;
pub(crate) use database::MemDatabase;

lazy_static! {
    static ref MEM_DB: MemDatabase = {
        let m = MemDatabase::new();
        m
    };
}

pub fn incr_offset(uuid: &str, offset: i64) {
    MEM_DB
        .tx
        .send(Message {
            event: Event::IncrOffset,
            pod: Pod {
                uuid: uuid.to_string(),
                last_offset: offset,
                ..Default::default()
            },
        })
        .unwrap()
}

pub fn apply(pod: &Pod) {
    MEM_DB
        .tx
        .send(Message {
            event: Event::Apply,
            pod: pod.clone(),
        })
        .unwrap();
}

pub fn delete(uuid: &str) {
    MEM_DB
        .tx
        .send(Message {
            event: Event::Delete,
            pod: Pod {
                uuid: uuid.to_string(),
                ..Default::default()
            },
        })
        .unwrap();
}

pub fn all_to_json() -> String {
    PodListMarshaller(
        MEM_DB
            .pods
            .read()
            .unwrap()
            .iter()
            .map(|(_, v)| v.clone())
            .collect::<Vec<Pod>>(),
    )
    .to_json()
}

pub fn close() {
    MEM_DB
        .tx
        .send(Message {
            event: Event::Close,
            pod: Pod {
                ..Default::default()
            },
        })
        .unwrap();
}

pub fn get(uuid: &str) -> Option<Pod> {
    match MEM_DB.pods.read() {
        Ok(pods) => match pods.get(uuid) {
            Some(pod) => Some(pod.clone()),
            None => None,
        },
        Err(_) => None,
    }
}

pub fn get_slice_with_ns_pod(ns: &str, pod: &str) -> Vec<(String, Pod)> {
    MEM_DB
        .pods
        .read()
        .unwrap()
        .iter()
        .filter(|(_, v)| v.ns == ns && v.pod == pod)
        .map(|(uuid, pod)| (uuid.clone(), pod.clone()))
        .collect::<Vec<(String, Pod)>>()
}

pub fn delete_with_ns_pod(pod_ns: &str, pod_name: &str) {
    MEM_DB
        .tx
        .send(Message {
            event: Event::Delete,
            pod: Pod {
                ns: pod_ns.to_string(),
                pod: pod_name.to_string(),
                ..Default::default()
            },
        })
        .unwrap();
}

pub fn pod_upload_stop(ns: &str, pod_name: &str) {
    let res = get_slice_with_ns_pod(ns, pod_name);
    for (_, mut pod) in res {
        pod.un_upload();
        pod.set_state_stop();
        apply(&pod);
    }
}

pub fn pod_upload_start(ns: &str, pod_name: &str) {
    let res = get_slice_with_ns_pod(ns, pod_name);
    for (_, mut pod) in res {
        pod.upload();
        pod.set_state_run();
        apply(&pod);
    }
}
