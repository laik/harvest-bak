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

    pub fn subscribe<F>(&mut self, name: String, listener: F)
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

    pub fn publish(&self, name: String, data: T) {
        if self.listeners.contains_key(&name) {
            for listener in self.listeners.get(&name).unwrap().iter() {
                listener(data.clone())
            }
        }
    }
}
