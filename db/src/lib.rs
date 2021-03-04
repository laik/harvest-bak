mod database;

pub use database::{new_sync_database, Database, Event, GetPod, Pod, State};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
