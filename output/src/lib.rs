use common::{Item, Result};

pub trait IOutput {
    fn write(&mut self, item: Item) -> Result<()>;
}

#[derive(Debug)]
pub struct Output<T: ?Sized + IOutput> {
    o: T,
}

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

pub fn via_output<T: IOutput>(line: &str, o: &mut T) -> Result<()> {
    o.write(Item::from(line))
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

    #[test]
    fn it_works() {
        let output = &mut Output::new(FakeOutput);
        if let Err(e) = via_output(&r#"abc"#, output) {
            panic!("{}", e);
        }
    }
}
