use std::collections::HashMap;
pub struct EventHandler<T: Clone> {
    listeners: HashMap<String, Vec<Box<dyn Fn(T) + Send + Sync>>>,
}

impl<T> EventHandler<T>
where
    T: Clone,
{
    pub fn new() -> EventHandler<T> {
        Self {
            listeners: HashMap::new(),
        }
    }

    pub fn registry<F>(&mut self, name: String, listener: F)
    where
        F: Fn(T) + Send + Sync + 'static,
    {
        let listener = Box::new(listener);
        if self.listeners.contains_key(&name) {
            self.listeners.get_mut(&name).unwrap().push(listener);
        } else {
            self.listeners.insert(name, vec![listener]);
        }
    }

    pub fn event(&self, name: String, data: T) {
        if self.listeners.contains_key(&name) {
            for listener in self.listeners.get(&name).unwrap().iter() {
                listener(data.clone())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::EventHandler;

    #[test]
    fn it_works() {
        let (tx, rx) = std::sync::mpsc::sync_channel(1);

        let tx1 = tx.clone();

        let mut event_handler = EventHandler::<String>::new();

        event_handler.registry("pod_name_update".to_owned(), move |s: String| {
            tx.send(s).unwrap();
        });

        event_handler.registry("pod_name_update2".to_owned(), move |s: String| {
            tx1.send(s).unwrap();
        });

        event_handler.event("pod_name_update".to_owned(), "abc".to_owned());
        assert_eq!(rx.recv().unwrap(), "abc");

        event_handler.event("pod_name_update2".to_owned(), "cde".to_owned());
        assert_eq!(rx.recv().unwrap(), "cde");
    }
}
