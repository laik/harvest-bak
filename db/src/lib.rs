pub use spin;

mod map;
mod table;

pub use map::*;
pub use table::Accessor;
mod database;

pub use database::{new_arc_database, Database, Event, GetPod, Pod, State};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
