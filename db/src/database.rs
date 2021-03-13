use super::Pod;
use crossbeam_channel::{unbounded, Sender};
use log::error as err;
use std::sync::RwLock;
use std::thread;
use std::{collections::HashMap, sync::Arc};
use strum::AsRefStr;

// #[derive(AsRefStr, Debug, Eq, Clone, PartialOrd)]
#[derive(AsRefStr, Debug, Clone)]
pub enum Event {
    #[strum(serialize = "apply")]
    Apply,
    #[strum(serialize = "del")]
    Delete,
    #[strum(serialize = "incr_offset")]
    IncrOffset,
    #[strum(serialize = "close")]
    Close,
}

unsafe impl Sync for Event {}
unsafe impl Send for Event {}

pub(crate) type UUID = String;

#[derive(Debug, Clone)]
pub struct Message {
    pub event: Event,
    pub pod: Pod,
}

pub struct MemDatabase {
    // pod key is the pod path uuid
    pub(crate) pods: Arc<RwLock<HashMap<UUID, Pod>>>,
    // internal event send queue
    pub(crate) tx: Sender<Message>,
}

impl MemDatabase {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn default() -> Self {
        let (tx, rx) = unbounded::<Message>();
        let hm = Arc::new(RwLock::new(HashMap::<UUID, Pod>::new()));
        let t_hm = Arc::clone(&hm);
        thread::spawn(move || {
            while let Ok(msg) = rx.recv() {
                let evt = msg.event;
                let pod = msg.pod;

                let mut m = match t_hm.write() {
                    Ok(m) => m,
                    Err(e) => {
                        err!("MemDatabase thread write hm failed, error:{:?}", e);
                        continue;
                    }
                };

                match evt {
                    Event::Apply => m
                        .entry(pod.uuid.to_string())
                        .or_insert(pod.clone())
                        .merge_with(&pod),
                    Event::Delete => {
                        if pod.namespace != "" {
                            m.retain(|_, v| {
                                !(pod.namespace == v.namespace || pod.pod_name == v.pod_name)
                            });

                            continue;
                        }

                        m.remove(&pod.uuid);
                    }
                    Event::IncrOffset => {
                        if let Some(_pod) = m.get_mut(&pod.uuid) {
                            _pod.offset += pod.incr_offset
                        };
                    }
                    Event::Close => {
                        break;
                    }
                };
            }
        });

        Self { pods: hm, tx }
    }
}

#[cfg(test)]
mod tests {
    use crate::Event;
    #[test]
    fn event_it_works() {
        assert_eq!(Event::Apply.as_ref(), "apply");
        assert_eq!(Event::Delete.as_ref(), "del");
    }
}
