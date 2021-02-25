use common::{Item, Result};
use std::sync::{Arc, Mutex};

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

pub fn sync_via_output<T: IOutput>(line: &str, output: Arc<Mutex<T>>) -> Result<()> {
    if let Ok(mut output) = output.lock() {
        return output.write(Item::from(line));
    }
    Ok(())
}

pub fn via_output<T: IOutput>(line: &str, o: &mut T) -> Result<()> {
    o.write(Item::from(line))
}

pub fn new_sync_output<T: IOutput>(t: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(t))
}

pub struct FakeOutput;

impl IOutput for FakeOutput {
    fn write(&mut self, item: Item) -> Result<()> {
        println!("{}", item.string());
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
        // let (tx,rx) = std::sync::mpsc::sync_channel(1);

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
}
