use crate::ConcHashMap;
use event::obj::Dispatch;
use event::Listener;
use serde::{Deserialize, Serialize};
use std::{
    hash::Hash,
    sync::{Arc, RwLock},
};
use strum::AsRefStr;

#[derive(Debug, Clone, Deserialize, Serialize)]
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

#[derive(Debug, Clone, Deserialize, Serialize)]
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
}

impl Default for Pod {
    fn default() -> Pod {
        Pod {
            uuid: "".to_owned(),
            offset: 0,
            namespace: "".to_owned(),
            pod_name: "".to_owned(),
            container_name: "".to_owned(),
            upload: false,
            state: State::Running,
            filter: "".to_owned(),
            output: "".to_owned(),
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
pub struct Database {
    // pod key is the pod path uuid
    pods: ConcHashMap<String, Pod>,
    // pod op registry and handle events
    event_dispatch: Dispatch<Pod>,
}

impl Database {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default() -> Self {
        Self {
            pods: ConcHashMap::<String, Pod>::new(),
            event_dispatch: Dispatch::<Pod>::new(),
        }
    }

    pub fn append_add_event<L>(&mut self, l: L)
    where
        L: Listener<Pod> + Send + Sync + 'static,
    {
        self.event_dispatch
            .registry(Event::Add.as_ref().to_string(), l)
    }

    pub fn append_delete_event<L>(&mut self, l: L)
    where
        L: Listener<Pod> + Send + Sync + 'static,
    {
        self.event_dispatch
            .registry(Event::Delete.as_ref().to_string(), l)
    }

    pub fn append_update_event<L>(&mut self, l: L)
    where
        L: Listener<Pod> + Send + Sync + 'static,
    {
        self.event_dispatch
            .registry(Event::Update.as_ref().to_string(), l)
    }

    pub fn all(&self) -> PodListMarshaller {
        PodListMarshaller(
            self.pods
                .iter()
                .map(|(_, v)| v.clone())
                .collect::<Vec<Pod>>(),
        )
    }

    pub fn get(&self, uuid: String) -> Option<&Pod> {
        match self.pods.find(&*uuid) {
            Some(v) => Some(v.get()),
            _ => None,
        }
    }

    pub fn incr_offset_by_uuid(&mut self, uuid: String, incr_size: i64) {
        if let Some(mut v) = self.pods.find_mut(&uuid) {
            v.get().offset += incr_size
        }
    }

    pub fn get_by_ns_pod(&self, ns: String, pod: String) -> Vec<Option<(String, Pod)>> {
        self.pods
            .iter()
            .map(|(k, v)| {
                if v.namespace == ns && v.pod_name == pod {
                    Some((k.clone(), v.clone()))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    }

    pub fn put(&mut self, pod: Pod) {
        if let Some(mut _pod) = self.pods.insert(pod.uuid.clone(), pod) {
            self.event_dispatch
                .dispatch(Event::Add.as_ref().to_string(), _pod.clone());
        }
    }

    pub fn delete_by_ns_pod(&mut self, ns: String, pod: String) {
        let need_deleted_list = self
            .pods
            .iter()
            .map(|(k, v)| {
                if v.namespace == ns && v.pod_name == pod {
                    self.pods.remove(k);
                    Some((k, v))
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();

        for item in need_deleted_list.iter() {
            if let Some((_, _pod)) = item {
                self.event_dispatch
                    .dispatch(Event::Delete.as_ref().to_string(), (**_pod).clone());
            }
        }
    }

    pub fn delete(&mut self, uuid: String) {
        if let Some(pod) = self.pods.remove(&*uuid) {
            self.event_dispatch
                .dispatch(Event::Delete.as_ref().to_string(), pod);
        }
    }

    pub fn update(&mut self, uuid: String, v: Pod) {
        if let Some(_pod) = self.pods.insert(uuid, v) {
            self.event_dispatch
                .dispatch(Event::Delete.as_ref().to_string(), _pod)
        }
    }
}

pub fn new_arc_database(db: Database) -> Arc<RwLock<Database>> {
    Arc::new(RwLock::new(db))
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
