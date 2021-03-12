use super::*;
use db::AMDB;
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
    // amdb: AMemDatabase,
}

impl Harvest {
    pub fn new(namespace: String, dir: String, api_server_addr: String, node_name: String) -> Self {
        Self {
            node_name,
            api_server_addr,
            scanner: Arc::new(RwLock::new(AutoScanner::new(namespace, dir))),
            // amdb: AMemDatabase::new(),
        }
    }

    pub fn start(&mut self) -> Result<()> {
        let frw = Arc::new(Mutex::new(FileReaderWriter::new(AMDB.clone(), 1000)));
        // registry db event handle
        let mut amdb = AMDB.clone();
        amdb.append_update_event(DBUpdateEvent(AMDB.clone(), frw.clone()));
        amdb.append_delete_event(DBDeleteEvent(AMDB.clone(), frw.clone()));
        amdb.append_add_event(DBAddEvent(AMDB.clone(), frw.clone()));

        match self.scanner.write() {
            Ok(mut scan) => {
                // registry scanner event handle
                scan.append_close_event_handle(ScannerCloseEvent(AMDB.clone(), frw.clone()));
                scan.append_open_event_handle(ScannerOpenEvent(AMDB.clone(), frw.clone()));
                scan.append_write_event_handle(ScannerWriteEvent(AMDB.clone(), frw.clone()));
            }
            _ => {}
        }

        let mut api_client = ApiClient::new(AMDB.clone());
        let jh2 = match api_client.watch(&self.api_server_addr, &self.node_name) {
            Ok(it) => it,
            Err(e) => return Err(e),
        };

        // start auto scanner with a new thread
        let scanner = self.scanner.clone();
        let jh = thread::spawn(move || match scanner.write() {
            Ok(mut scan) => {
                if let Ok(res) = scan.prepare_scan() {
                    if res.len() < 1 {
                        return;
                    }

                    for item in res.iter() {
                        amdb.put(item.to_pod())
                    }
                }
                apply_rules();

                if let Err(e) = scan.watch_start() {
                    eprintln!("{}", e);
                }
            }
            Err(e) => {
                eprintln!("{}", e);
            }
        });

        let cfg = Config::build(Environment::Production)
            .address("0.0.0.0")
            .port(8080)
            .unwrap();

        rocket::custom(cfg)
            .mount("/", routes![post_pod, query_pod, query_rules])
            .register(catchers![not_found])
            .manage(AMDB.clone())
            .launch();

        jh.join().unwrap();
        jh2.join().unwrap();

        Ok(())
    }
}
