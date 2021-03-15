use super::*;
use file::FileReaderWriter;
use rocket::config::{Config, Environment};
use rocket::routes;
use scan::AutoScanner;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

pub struct Harvest<'a> {
    node_name: &'a str,
    namespace: &'a str,
    dir: &'a str,
    api_server_addr: &'a str,
}

impl<'a> Harvest<'a> {
    pub fn new(
        namespace: &'a str,
        dir: &'a str,
        api_server_addr: &'a str,
        node_name: &'a str,
    ) -> Self {
        Self {
            namespace,
            dir,
            node_name,
            api_server_addr,
        }
    }

    pub fn start(&mut self, num_workers: usize) -> Result<()> {
        let scanner = Arc::new(RwLock::new(AutoScanner::new(
            String::from(self.namespace),
            String::from(self.dir),
        )));
        let frw = Arc::new(Mutex::new(FileReaderWriter::new(num_workers)));

        if let Ok(mut scan) = scanner.write() {
            // registry scanner event handle
            scan.append_open_event_handle(ScannerOpenEvent());
            scan.append_write_event_handle(ScannerWriteEvent(frw.clone()));
            scan.append_close_event_handle(ScannerCloseEvent());
        }

        db::registry_open_event_listener(DBOpenEvent(frw.clone()));
        db::registry_open_event_listener(DBCloseEvent(frw.clone()));

        let mut threads = vec![];
        // start auto scanner with a new thread
        threads.push(thread::spawn(move || {
            let mut scan = match scanner.write() {
                Ok(it) => it,
                Err(e) => {
                    eprintln!("{}", e);
                    return;
                }
            };

            let res = match scan.prepare_scan() {
                Ok(it) => it,
                Err(e) => {
                    eprintln!("{}", e);
                    return;
                }
            };

            // add to local MemDatabase
            for item in res.iter() {
                db::insert(&item.to_pod())
            }

            if let Err(e) = scan.directory_watch_start() {
                eprintln!("{}", e);
            }
        }));

        threads.push(thread::spawn(move || {
            let cfg = Config::build(Environment::Development)
                .address("0.0.0.0")
                .port(8080)
                .unwrap();

            rocket::custom(cfg)
                .mount("/", routes![post_pod, query_pod, query_rules])
                .register(catchers![not_found])
                .launch();
        }));

        let api_server_addr = self.api_server_addr.to_string().clone();
        let node_name = self.node_name.to_string().clone();
        recv_tasks(&api_server_addr, &node_name);

        for item in threads {
            item.join().unwrap();
        }

        Ok(())
    }
}
