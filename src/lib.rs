mod db;
mod event_handler;
mod server;

pub use db::Database;
pub use event_handler::EventHandler;
pub use server::RpcServer;

pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
