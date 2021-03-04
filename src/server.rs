use super::*;
use db::{Database, Pod};
use event::obj::{Dispatch, Listener};
use file::FileReaderWriter;
use rocket::config::{Config, Environment};
use rocket::State;
use rocket::{get, post, routes};
use rocket_contrib::json::{Json, JsonValue};
use scan::{AutoScanner, ScannerRecvArgument};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;

pub struct Harvest {
    scanner: AutoScanner,
    database: Arc<Mutex<Database>>,
    file_reader_writer: Arc<Mutex<FileReaderWriter>>,
    // jh: Option<JoinHandle<Result<()>>>,
    db_handles: HashMap<String, Box<dyn Listener<Pod> + 'static>>,
    scanner_handles:
        Arc<Mutex<HashMap<String, Box<dyn Listener<ScannerRecvArgument> + Send + 'static>>>>,
}

impl Harvest {
    pub fn new(namespace: String, dir: String) -> Self {
        let db_event_handler = Dispatch::<Pod>::new();
        let database = Arc::new(Mutex::new(Database::new(db_event_handler)));

        let file_reader_db = database.clone();
        Self {
            scanner: AutoScanner::new(namespace, dir, database.clone()),
            database,
            file_reader_writer: Arc::new(Mutex::new(FileReaderWriter::new(file_reader_db))),
            // jh: None,
            db_handles: HashMap::new(),
            scanner_handles: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn start(&mut self) -> Result<()> {
        let cfg = Config::build(Environment::Development)
            .address("0.0.0.0")
            .port(8080)
            .unwrap();

        let database = self.database.clone();

        // registry db event handle
        let db_update_handle_arc_clone = self.database.clone();
        let fwr_update_handle_arc_clone = self.file_reader_writer.clone();
        self.db_handles.insert(
            db::Event::Update.as_ref().to_owned(),
            Box::new(DBUpdateEvent(
                db_update_handle_arc_clone,
                fwr_update_handle_arc_clone,
            )),
        );

        let db_delete_handle_arc_clone = self.database.clone();
        let fwr_delete_handle_arc_clone = self.file_reader_writer.clone();
        self.db_handles.insert(
            db::Event::Delete.as_ref().to_owned(),
            Box::new(DBDeleteEvent(
                db_delete_handle_arc_clone,
                fwr_delete_handle_arc_clone,
            )),
        );

        let db_add_handle_arc_clone = self.database.clone();
        let fwr_add_handle_arc_clone = self.file_reader_writer.clone();
        self.db_handles.insert(
            db::Event::Add.as_ref().to_owned(),
            Box::new(DBAddEvent(
                db_add_handle_arc_clone,
                fwr_add_handle_arc_clone,
            )),
        );

        let scanner_clone = self.scanner.clone();
        let self_scanner_handles_registry = self.scanner_handles.clone();

        match self_scanner_handles_registry.lock() {
            Ok(mut scanner_event_handler) => {
                // registry scanner event handle
                let scanner_need_update_handler_db_arc = database.clone();
                let scanner_need_update_handler_frw_arc = self.file_reader_writer.clone();
                scanner_event_handler.insert(
                    scan::PathEvent::NeedClose.as_ref().to_owned(),
                    Box::new(ScannerCloseEvent(
                        scanner_need_update_handler_db_arc,
                        scanner_need_update_handler_frw_arc,
                    )),
                );

                let scanner_need_open_handler_db_arc = database.clone();
                let scanner_need_open_handler_frw_arc = self.file_reader_writer.clone();
                scanner_event_handler.insert(
                    scan::PathEvent::NeedOpen.as_ref().to_owned(),
                    Box::new(ScannerOpenEvent(
                        scanner_need_open_handler_db_arc,
                        scanner_need_open_handler_frw_arc,
                    )),
                );

                let scanner_need_write_handler_db_arc = database.clone();
                let scanner_need_write_handler_frw_arc = self.file_reader_writer.clone();
                scanner_event_handler.insert(
                    scan::PathEvent::NeedWrite.as_ref().to_owned(),
                    Box::new(ScannerWriteEvent(
                        scanner_need_write_handler_db_arc,
                        scanner_need_write_handler_frw_arc,
                    )),
                );
            }
            Err(e) => {
                panic!("registry scanner handle error: {:?}", e)
            }
        }

        let self_scanner_handles = self.scanner_handles.clone();
        // start auto scanner with a new thread
        let _jh = thread::spawn(move || scanner_clone.start(self_scanner_handles));

        rocket::custom(cfg)
            .mount("/", routes![post_pod, query_pod])
            .register(catchers![not_found])
            .manage(database)
            .launch();

        if let Err(e) = _jh.join().unwrap() {
            eprintln!("{}", e)
        }

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
fn post_pod(req: Json<Request>, db: State<'_, Arc<Mutex<Database>>>) -> JsonValue {
    if req.0.namespace == "" || req.0.pod == "" {
        return json!({
            "status": "error",
            "reason": format!("namespace {} or pod {} maybe is empty",req.namespace,req.pod),
        });
    }

    match db.lock() {
        Ok(mut db) => {
            for item in db.get_by_namespace_pod(req.0.namespace, req.0.pod).iter() {
                if let Some((uuid, pod)) = item {
                    let mut pod = pod.to_owned();
                    pod.upload = req.0.upload;
                    pod.filter = req.0.filter.clone();
                    pod.output = req.0.output.clone();
                    match db.update(uuid.to_owned(), pod) {
                        Ok(_) => {}
                        Err(e) => {
                            eprintln!("{}", e);
                            return json!({"status":"error","reason":format!("{}",e)});
                        }
                    }
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
fn query_pod(db: State<'_, Arc<Mutex<Database>>>) -> JsonValue {
    match db.lock() {
        Ok(_db) => {
            json!({"status":"ok","reason":format!("{:?}",_db.all().to_json())})
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
