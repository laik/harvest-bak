#![feature(proc_macro_hygiene, decl_macro)]
#![feature(trait_alias)]
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate rocket;

mod event_listener;
mod server;

pub use common::Result;
pub(crate) use event_listener::{
    DBAddEvent, DBDeleteEvent, DBUpdateEvent, ScannerCloseEvent, ScannerOpenEvent,
    ScannerWriteEvent,
};
pub use server::Harvest;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
