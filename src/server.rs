use super::Result;
use db::{Database, GetPod, Pod};
use event::obj::{Dispatch, Listener};
use file::FileReader;
use rocket::config::{Config, Environment};
use rocket::State;
use rocket::{delete, get, post, routes};
use rocket_contrib::json::{Json, JsonValue};
use scan::{AutoScanner, GetPathEventInfo, PathEventInfo, ScannerRecvArgument};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::RandomState;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread;
use std::{collections::HashMap, thread::JoinHandle};

struct DBAddEvent(Arc<Mutex<Database>>);
impl<T> Listener<T> for DBAddEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        if let Some(pod) = t.get() {
            println!("db add pod {:?}", pod);
        }
    }
}

struct DBDeleteEvent(Arc<Mutex<Database>>);
impl<T> Listener<T> for DBDeleteEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        if let Some(pod) = t.get() {
            println!("db delete pod {:?}", pod);
        }
    }
}

struct DBUpdateEvent(Arc<Mutex<Database>>);
impl<T> Listener<T> for DBUpdateEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        if let Some(pod) = t.get() {
            println!("db update pod {:?}", pod);
        }
    }
}

struct ScannerWriteEvent(Arc<Mutex<Database>>);
impl<T> Listener<T> for ScannerWriteEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        println!("{:?}", t.get().unwrap())
    }
}

struct ScannerOpenEvent(Arc<Mutex<Database>>);
impl<T> Listener<T> for ScannerOpenEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let pei = t.get().unwrap();
        if let Ok(mut db) = self.0.lock() {
            if let Err(e) = db.put(pei.to_pod()) {
                eprintln!("{}", e)
            }
        }
    }
}

struct ScannerCloseEvent(Arc<Mutex<Database>>);
impl<T> Listener<T> for ScannerCloseEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {
        let pei = t.get().unwrap();
        if let Ok(mut db) = self.0.lock() {
            if let Err(e) = db.delete(pei.path.to_owned()) {
                eprintln!("{}", e)
            }
        }
    }
}

pub struct Harvest {
    scanner: AutoScanner,
    database: Arc<Mutex<Database>>,
    file_reader: FileReader,
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
            file_reader: FileReader::new(file_reader_db),
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
        self.db_handles.insert(
            db::Event::Update.as_ref().to_owned(),
            Box::new(DBUpdateEvent(db_update_handle_arc_clone)),
        );

        let db_delete_handle_arc_clone = self.database.clone();
        self.db_handles.insert(
            db::Event::Delete.as_ref().to_owned(),
            Box::new(DBDeleteEvent(db_delete_handle_arc_clone)),
        );

        let db_add_handle_arc_clone = self.database.clone();
        self.db_handles.insert(
            db::Event::Add.as_ref().to_owned(),
            Box::new(DBAddEvent(db_add_handle_arc_clone)),
        );

        let scanner_clone = self.scanner.clone();
        let self_scanner_handles_registry = self.scanner_handles.clone();

        match self_scanner_handles_registry.lock() {
            Ok(mut scanner_event_handler) => {
                // registry scanner event handle
                let scanner_need_update_handler_db_arc = database.clone();
                scanner_event_handler.insert(
                    scan::PathEvent::NeedClose.as_ref().to_owned(),
                    Box::new(ScannerCloseEvent(scanner_need_update_handler_db_arc)),
                );

                let scanner_need_open_handler_db_arc = database.clone();
                scanner_event_handler.insert(
                    scan::PathEvent::NeedOpen.as_ref().to_owned(),
                    Box::new(ScannerOpenEvent(scanner_need_open_handler_db_arc)),
                );

                let scanner_need_write_handler_db_arc = database.clone();
                scanner_event_handler.insert(
                    scan::PathEvent::NeedWrite.as_ref().to_owned(),
                    Box::new(ScannerWriteEvent(scanner_need_write_handler_db_arc)),
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
            .mount("/", routes![apply_pod, delete_pod, query_pod])
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
    container: String,
}

#[post("/pod", format = "json", data = "<req>")]
fn apply_pod(req: Json<Request>, db: State<'_, Arc<Mutex<Database>>>) -> JsonValue {
    if req.0.namespace == "" || req.0.pod == "" {
        return json!({
            "status": "error",
            "reason": format!("namespace {} or pod {} maybe is empty",req.namespace,req.pod),
        });
    }

    match db.lock() {
        Ok(mut db) => {
            if let Err(e) = db.put(Pod {
                uuid: "".to_string(),
                offset: 0,
                upload: true,
                namespace: req.0.namespace,
                pod_name: req.0.pod,
                container_name: req.0.container,
            }) {
                return json!({"status":"error","reason":format!("{}",e)});
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

#[delete("/pod", format = "json", data = "<req>")]
fn delete_pod(req: Json<Request>, db: State<'_, Arc<Mutex<Database>>>) -> JsonValue {
    match db.lock() {
        Ok(mut _db) => {
            if let Err(e) = _db.delete_by_namespace_pod(req.0.namespace, req.0.pod) {
                return json!({"status":"error","reason":format!("{:?}",e)});
            }
            json!({"status":"ok","reason":"none"})
        }
        Err(e) => {
            json!({"status":"error","reason":format!("{}",e)})
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
