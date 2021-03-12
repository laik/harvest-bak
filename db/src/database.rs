use event::obj::Dispatch;
use event::Listener;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::hash::Hash;
use strum::AsRefStr;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum State {
    Ready,
    Running,
    Stopped,
}
#[derive(AsRefStr, Debug, Eq, Clone, PartialOrd)]
pub enum Event {
    #[strum(serialize = "add")]
    Add,
    #[strum(serialize = "del")]
    Delete,
    #[strum(serialize = "update")]
    Update,
}

impl Hash for Event {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match *self {
            Event::Add => Event::Add.hash(state),
            Event::Delete => Event::Delete.hash(state),
            Event::Update => Event::Update.hash(state),
        }
    }
}

impl PartialEq for Event {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Event::Add, Event::Add) => true,
            (Event::Delete, Event::Delete) => true,
            (Event::Update, Event::Update) => true,
            _ => false,
        }
    }
}

unsafe impl Sync for Event {}
unsafe impl Send for Event {}

pub trait GetPod {
    fn get(&self) -> Option<&Pod>;
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Pod {
    // on this the uuid path is unique identifier
    pub uuid: String,
    pub offset: i64,
    pub namespace: String,
    pub pod_name: String,
    pub container_name: String,
    pub upload: bool,
    pub state: State,
    pub filter: String,
    pub output: String,
    pub ips: Vec<String>,
}

impl Pod {
    pub fn set_running(&mut self) {
        self.state = State::Running;
    }
    pub fn set_stopped(&mut self) {
        self.state = State::Stopped;
    }
    pub fn upload(&mut self) {
        self.upload = true;
    }
    pub fn unupload(&mut self) {
        self.upload = false;
    }
    pub(crate) fn merge_with(&mut self, other: &Pod) {
        self.upload = other.upload;
        self.filter = other.clone().filter;
        self.output = other.clone().output;
    }
}

impl Default for Pod {
    fn default() -> Pod {
        Pod {
            uuid: "".into(),
            offset: 0,
            namespace: "".into(),
            pod_name: "".into(),
            container_name: "".into(),
            upload: false,
            state: State::Running,
            filter: "".into(),
            output: "".into(),
            ips: Vec::new(),
        }
    }
}

impl GetPod for Pod {
    fn get(&self) -> Option<&Pod> {
        Some(self)
    }
}

type PodList = Vec<Pod>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PodListMarshaller(PodList);

impl PodListMarshaller {
    pub fn to_json(&self) -> String {
        match serde_json::to_string(&self.0) {
            Ok(contents) => contents,
            Err(_) => "".to_owned(),
        }
    }
}

pub(crate) type UUID = String;

pub(crate) struct MemDatabase {
    // pod key is the pod path uuid
    pub(crate) pods: HashMap<UUID, Pod>,
    // pod op registry and handle events
    event_dispatch: Dispatch<Pod>,
}

impl MemDatabase {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn default() -> Self {
        Self {
            pods: HashMap::<UUID, Pod>::new(),
            event_dispatch: Dispatch::<Pod>::new(),
        }
    }

    pub(crate) fn append_add_event<L>(&mut self, l: L)
    where
        L: Listener<Pod> + Send + Sync + 'static,
    {
        self.event_dispatch
            .registry(Event::Add.as_ref().to_string(), l)
    }

    pub(crate) fn append_delete_event<L>(&mut self, l: L)
    where
        L: Listener<Pod> + Send + Sync + 'static,
    {
        self.event_dispatch
            .registry(Event::Delete.as_ref().to_string(), l)
    }

    pub(crate) fn append_update_event<L>(&mut self, l: L)
    where
        L: Listener<Pod> + Send + Sync + 'static,
    {
        self.event_dispatch
            .registry(Event::Update.as_ref().to_string(), l)
    }

    pub(crate) fn all(&self) -> PodListMarshaller {
        PodListMarshaller(
            self.pods
                .iter()
                .map(|(_, v)| v.clone())
                .collect::<Vec<Pod>>(),
        )
    }

    pub(crate) fn incr_offset_by_uuid(&mut self, uuid: String, incr_size: i64) {
        if let Some(mut v) = self.pods.get_mut(&uuid) {
            v.offset += incr_size
        }
    }

    pub(crate) fn get_slice_by_ns_pod(&self, ns: String, pod: String) -> Vec<(String, Pod)> {
        let result = self
            .pods
            .iter()
            .filter(|(_, v)| v.namespace == ns && v.pod_name == pod)
            .map(|(uuid, pod)| (uuid.clone(), pod.clone()))
            .collect::<Vec<(String, Pod)>>();
        result
    }

    pub(crate) fn apply(&mut self, pod: &Pod) {
        self.pods
            .entry((*pod).uuid.to_string())
            .or_insert(pod.clone())
            .merge_with(pod)
    }

    pub(crate) fn put(&mut self, pod: Pod) {
        self.pods.insert(pod.uuid.clone(), pod.clone());
        self.dispatch_add(pod.clone());
    }

    pub(crate) fn delete_by_ns_pod(&mut self, ns: String, pod_name: String) {
        let need_delete_list = self
            .pods
            .iter()
            .filter(|(_, pod)| pod.namespace == ns || pod.pod_name == pod_name)
            .map(|(_, v)| v.clone())
            .collect::<Vec<Pod>>();

        for pod in need_delete_list.iter() {
            self.pods.remove(&pod.uuid);
        }

        for pod in need_delete_list.iter() {
            self.dispatch_delete(pod.clone());
        }
    }

    pub(crate) fn delete(&mut self, uuid: String) {
        if let Some(pod) = self.pods.remove(&*uuid) {
            self.dispatch_delete(pod);
        }
    }

    fn dispatch_update(&mut self, pod: Pod) {
        self.event_dispatch
            .dispatch(Event::Update.as_ref().to_string(), pod);
    }

    fn dispatch_delete(&mut self, pod: Pod) {
        self.event_dispatch
            .dispatch(Event::Delete.as_ref().to_string(), pod);
    }

    fn dispatch_add(&mut self, pod: Pod) {
        self.event_dispatch
            .dispatch(Event::Add.as_ref().to_string(), pod);
    }

    pub(crate) fn stop_upload_pod(&mut self, ns: String, pod_name: String) {
        let res = self.get_slice_by_ns_pod(ns.clone(), pod_name.clone());
        for (uuid, pod) in res.iter() {
            let mut pod = pod.clone();
            pod.unupload();
            pod.set_stopped();
            self.pods.insert(uuid.clone(), pod.clone());
            self.dispatch_update(pod);
        }
    }

    pub(crate) fn start_upload_pod(&mut self, ns: String, pod_name: String) {
        let res = self.get_slice_by_ns_pod(ns.clone(), pod_name.clone());
        for (uuid, pod) in res.iter() {
            let mut pod = pod.clone();
            pod.upload();
            pod.set_running();
            self.pods.insert(uuid.clone(), pod.clone());
            self.dispatch_update(pod);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::Event;

    #[test]
    fn event_it_works() {
        assert_eq!(Event::Add.as_ref(), "add");
        assert_eq!(Event::Delete.as_ref(), "del");
        assert_eq!(Event::Update.as_ref(), "update");
    }
}
