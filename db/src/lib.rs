#[warn(mutable_borrow_reservation_conflict)]
mod database;

pub use database::{new_arc_database, Event, GetPod, MemDatabase, Pod, State};

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
