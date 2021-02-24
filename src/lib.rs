#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket_contrib;
#[macro_use]
extern crate rocket;

mod server;

pub use common::Result;
pub use server::Harvest;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
