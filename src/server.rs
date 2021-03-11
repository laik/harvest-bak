use super::*;
use db::MemDatabase;
use file::FileReaderWriter;
use rocket::config::{Config, Environment};
use rocket::routes;
use scan::AutoScanner;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

pub struct Harvest {
    node_name: String,
    api_server_addr: String,
    scanner: Arc<RwLock<AutoScanner>>,
    database: Arc<RwLock<MemDatabase>>,
}

impl Harvest {
    pub fn new(namespace: String, dir: String, api_server_addr: String, node_name: String) -> Self {
        Self {
            node_name,
            api_server_addr,
            scanner: Arc::new(RwLock::new(AutoScanner::new(namespace, dir))),
            database: Arc::new(RwLock::new(MemDatabase::default())),
        }
    }

    pub fn start(&mut self) -> Result<()> {
        let cfg = Config::build(Environment::Development)
            .address("0.0.0.0")
            .port(8080)
            .unwrap();

        let frw = Arc::new(Mutex::new(FileReaderWriter::new(self.database.clone())));

        // registry db event handle
        match self.database.write() {
            Ok(mut rwdb) => {
                rwdb.append_update_event(DBUpdateEvent(self.database.clone(), frw.clone()));
                rwdb.append_delete_event(DBDeleteEvent(self.database.clone(), frw.clone()));
                rwdb.append_add_event(DBAddEvent(self.database.clone(), frw.clone()));
            }
            _ => {}
        }

        match self.scanner.write() {
            Ok(mut scan) => {
                // registry scanner event handle
                scan.append_close_event_handle(ScannerCloseEvent(
                    self.database.clone(),
                    frw.clone(),
                ));
                scan.append_open_event_handle(ScannerOpenEvent(self.database.clone(), frw.clone()));

                scan.append_write_event_handle(ScannerWriteEvent(
                    self.database.clone(),
                    frw.clone(),
                ));
            }
            _ => {}
        }

        // start auto scanner with a new thread
        let scanner = self.scanner.clone();
        let database = self.database.clone();
        let _jh = thread::spawn(move || match scanner.write() {
            Ok(mut scan) => {
                if let Ok(res) = scan.prepare_scan() {
                    if res.len() < 1 {
                        return;
                    }
                    if let Ok(mut db) = database.write() {
                        for item in res.iter() {
                            db.put(item.to_pod())
                        }
                    }
                }

                if let Err(e) = scan.watch_start() {
                    eprintln!("{}", e);
                }
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        });

        let mut api_client = ApiClient::new(self.database.clone());
        let jh2 = match api_client.watch(&self.api_server_addr, &self.node_name) {
            Ok(it) => it,
            Err(e) => return Err(e),
        };
        let database = self.database.clone();
        rocket::custom(cfg)
            .mount("/", routes![post_pod, query_pod])
            .register(catchers![not_found])
            .manage(database)
            .launch();

        _jh.join().unwrap();
        jh2.join().unwrap();

        Ok(())
    }
}
