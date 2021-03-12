use common::{Item, Result};
use log::{error as err, warn};
use once_cell::sync::Lazy;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use std::sync::atomic::{AtomicUsize, Ordering};

pub use OUTPUTS as OTS;

pub static OUTPUTS: Lazy<Arc<Mutex<Outputs>>> = Lazy::new(|| {
    let outputs = Arc::new(Mutex::new(Outputs::new()));
    if let Ok(mut ots) = outputs.lock() {
        ots.registry_output("fake_output".into(), Output::new(FakeOutput));
        ots.registry_output(
            "counter_output".into(),
            Output::new(Counter(AtomicUsize::new(0))),
        );
    }
    outputs
});

pub struct Outputs {
    output_listener: HashMap<String, Box<dyn IOutput>>,
}

impl Outputs {
    pub fn new() -> Self {
        Self {
            output_listener: HashMap::new(),
        }
    }

    pub fn registry_output<T>(&mut self, name: String, t: T)
    where
        T: IOutput + Send + Sync + 'static,
    {
        if self.output_listener.contains_key(&name) {
            return;
        }
        self.output_listener.insert(name, Box::new(t));
    }

    pub fn output(&mut self, name: String, line: &str) {
        if !self.output_listener.contains_key(&name) {
            if line.len() == 0 {
                return;
            }
            warn!("outputs not found output name `{}`", name);
            warn!("use default stdout {}", line);
            return;
        }

        match self.output_listener.get_mut(&name) {
            Some(o) => {
                if let Err(e) = o.write(Item::from(line)) {
                    err!("{:?}", e);
                }
            }
            _ => {}
        }
    }
}

pub trait IOutput: Send + Sync + 'static {
    fn write(&mut self, item: Item) -> Result<()>;
}

#[derive(Debug)]
pub struct Output<T: ?Sized + IOutput> {
    o: T,
}
unsafe impl<T: IOutput> Send for Output<T> {}
unsafe impl<T: IOutput> Sync for Output<T> {}

impl<T: IOutput> Output<T> {
    pub fn new(o: T) -> Self {
        Self { o }
    }
}

impl<T: IOutput> IOutput for Output<T> {
    // delegate write IOutput.write
    fn write(&mut self, item: Item) -> Result<()> {
        self.o.write(item)
    }
}

pub fn sync_via_output(line: &str, output: Arc<Mutex<dyn IOutput>>) -> Result<()> {
    if let Ok(mut output) = output.lock() {
        return output.write(Item::from(line));
    }
    Ok(())
}

pub fn via_output<'a, T: IOutput>(line: &'a str, o: &'a mut T) -> Result<()> {
    if line.len() == 0 {
        return Ok(());
    }
    o.write(Item::from(line))
}

pub fn new_sync_output<T: IOutput>(t: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(t))
}

pub struct FakeOutput;

impl IOutput for FakeOutput {
    fn write(&mut self, item: Item) -> Result<()> {
        println!("FakeOutput content: {:?}", item.string());
        Ok(())
    }
}

pub struct Counter(AtomicUsize);
impl IOutput for Counter {
    fn write(&mut self, item: Item) -> Result<()> {
        self.0.fetch_add(1, Ordering::SeqCst);
        if self.0.load(Ordering::Relaxed) as i64 % 10000 == 0 {
            println!("Kafka counter {:?}", self.0.load(Ordering::Relaxed));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    fn it_works() {
        let output = &mut Output::new(FakeOutput);
        if let Err(e) = via_output(&r#"abc"#, output) {
            panic!("{}", e);
        }
    }

    #[test]
    fn it_works_with_multiple_threads() {
        let fake_output = Arc::new(Mutex::new(FakeOutput));

        let mut list = vec![];
        for _ in 0..2 {
            let output = fake_output.clone();
            list.push(thread::spawn(move || {
                if let Err(e) = sync_via_output(&r#"abc"#, output) {
                    panic!("{}", e);
                }
            }));
        }

        for j in list.into_iter() {
            j.join().unwrap()
        }
    }

    #[test]
    fn it_works_with_outputs() {
        let mut outputs = Outputs::new();
        outputs.registry_output("fake_output".to_owned(), Output::new(FakeOutput));
        outputs.output("fake_output".to_owned(), "123")
    }

    #[test]
    fn it_static_outputs() {
        if let Ok(mut ots) = OUTPUTS.try_lock() {
            ots.output("fake_output".to_owned(), "1")
        }
        let mut j = vec![];
        let o1 = OUTPUTS.clone();
        j.push(thread::spawn(move || {
            if let Ok(mut ots) = o1.try_lock() {
                ots.output("fake_output".to_owned(), "2")
            }
        }));

        let o2 = OUTPUTS.clone();
        j.push(thread::spawn(move || {
            if let Ok(mut ots) = o2.try_lock() {
                ots.output("fake_output".to_owned(), "3")
            }
        }));

        let _ = j.into_iter().map(|_j| _j.join().unwrap());
    }
}
