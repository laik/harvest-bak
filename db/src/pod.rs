use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum State {
    Ready,
    Running,
    Stopped,
}
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Pod {
    pub uuid: String, // on this the uuid path is unique identifier
    pub offset: i64,
    pub namespace: String,
    pub pod_name: String,
    pub container_name: String,
    pub upload: bool,
    pub state: State,
    pub filter: String,
    pub output: String,
    pub ips: Vec<String>,
    pub incr_offset: i64,
}

impl Pod {
    pub fn set_running(&mut self) {
        self.state = State::Running;
    }
    pub fn set_ready(&mut self) {
        self.state = State::Stopped;
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
            state: State::Ready,
            filter: "".into(),
            output: "".into(),
            ips: Vec::new(),
            incr_offset: 0,
        }
    }
}

pub trait GetPod {
    fn get(&self) -> Option<&Pod>;
}

impl GetPod for Pod {
    fn get(&self) -> Option<&Pod> {
        Some(self)
    }
}

pub type PodList = Vec<Pod>;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct PodListMarshaller(pub PodList);

impl PodListMarshaller {
    pub fn to_json(&self) -> String {
        match serde_json::to_string(&self.0) {
            Ok(contents) => contents,
            Err(_) => "".to_owned(),
        }
    }
}
