use super::*;
use db::{Database, Pod};
use event::obj::{Dispatch, Listener};
use file::FileReaderWriter;
use output::FakeOutput;
use rocket::config::{Config, Environment};
use rocket::State;
use rocket::{get, post, routes};
use rocket_contrib::json::{Json, JsonValue};
use scan::{AutoScanner, ScannerRecvArgument};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

fn new_arc_mutex<T>(t: T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(t))
}

pub struct Harvest {
    scanner: Arc<RwLock<AutoScanner>>,
    database: Arc<RwLock<Database>>,
}

impl Harvest {
    pub fn new(namespace: String, dir: String) -> Self {
        Self {
            scanner: Arc::new(RwLock::new(AutoScanner::new(namespace, dir))),
            database: Arc::new(RwLock::new(Database::default())),
        }
    }

    pub fn start(&mut self) -> Result<()> {
        let cfg = Config::build(Environment::Development)
            .address("0.0.0.0")
            .port(8080)
            .unwrap();

        let frw = Arc::new(Mutex::new(FileReaderWriter::new(self.database.clone())));

        // register output
        if let Ok(mut _frw) = frw.lock() {
            _frw.set_output("fake_output".to_owned(), new_arc_mutex(FakeOutput));
        }

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

        let database = self.database.clone();
        rocket::custom(cfg)
            .mount("/", routes![post_pod, query_pod])
            .register(catchers![not_found])
            .manage(database)
            .launch();

        _jh.join().unwrap();

        Ok(())
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Request {
    namespace: String,
    pod: String,
    filter: String,
    output: String,
    upload: bool,
}

// /pod/collect list ns.pod start collect to output
#[post("/pod", format = "json", data = "<req>")]
fn post_pod(req: Json<Request>, db: State<'_, Arc<RwLock<Database>>>) -> JsonValue {
    if req.0.namespace == "" || req.0.pod == "" {
        return json!({
            "status": "error",
            "reason": format!("namespace {} or pod {} maybe is empty",req.namespace,req.pod),
        });
    }

    match db.write() {
        Ok(mut db) => {
            for item in db.get_by_ns_pod(req.0.namespace, req.0.pod).iter() {
                if let Some((uuid, pod)) = item {
                    let mut pod = pod.to_owned();
                    pod.upload = req.0.upload;
                    pod.filter = req.0.filter.clone();
                    pod.output = req.0.output.clone();
                    db.update(uuid.to_owned(), pod);
                }
            }
            json!({"status":"ok"})
        }
        Err(e) => {
            json!({
            "status":"error",
            "reason":format!("DB Lock Failure error {}",e)
            })
        }
    }
}

#[get("/pod")]
fn query_pod(db: State<'_, Arc<RwLock<Database>>>) -> JsonValue {
    match db.read() {
        Ok(_db) => {
            json!({"status":"ok","wreason":format!("{:?}",_db.all().to_json())})
        }
        Err(e) => {
            json!({"status":"error","reason":format!("{}",e)})
        }
    }
}

#[catch(404)]
fn not_found() -> JsonValue {
    json!({
        "status": "error",
        "reason": "Resource was not found."
    })
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
