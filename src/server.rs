use super::Result;
use db::{Database, GetPod, Pod};
use file::FileReader;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    thread::JoinHandle,
};

use rocket::config::{Config, Environment};
use rocket::State;
use rocket::{delete, get, post, routes};
use rocket_contrib::json::{Json, JsonValue};
use serde::{Deserialize, Serialize};

use event::obj::{Dispatch, Listener};

use scan::{AutoScanner, GetPathEventInfo, ScannerRecvArgument};
use std::thread;
struct DBAddEvent(Arc<Mutex<Database>>);
impl<T> Listener<T> for DBAddEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        println!("{:?}", t.get().unwrap());
    }
}

struct DBDeleteEvent(Arc<Mutex<Database>>);
impl<T> Listener<T> for DBDeleteEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        println!("{:?}", t.get().unwrap());
    }
}

struct DBUpdateEvent(Arc<Mutex<Database>>);
impl<T> Listener<T> for DBUpdateEvent
where
    T: Clone + GetPod,
{
    fn handle(&self, t: T) {
        println!("{:?}", t.get().unwrap());
    }
}

struct ScannerWriteEvent(Arc<Mutex<Database>>);
impl<T> Listener<T> for ScannerWriteEvent
where
    T: Clone + GetPathEventInfo,
{
    fn handle(&self, t: T) {}
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
            if let Err(e) = db.delete(pei.path.to_owned())
            {
                eprintln!("{}", e)
            }
        }
    }
}

pub struct Harvest {
    scanner: AutoScanner,
    database: Arc<Mutex<Database>>,
    file_reader: FileReader,
    _jh: Option<JoinHandle<Result<()>>>,
    db_handles: HashMap<String, Box<dyn Listener<Pod> + 'static>>,
    scan_handles: Arc<
        Mutex<
            HashMap<
                String,
                Box<
                    dyn Listener<(scan::PathEventInfo, Arc<Mutex<Database>>)>
                        + Send
                        + Sync
                        + 'static,
                >,
            >,
        >,
    >,
}

impl Harvest {
    pub fn new(namespace: String, dir: String) -> Self {
        let db_event_handler = Dispatch::<Pod>::new();
        let database = Arc::new(Mutex::new(Database::new(db_event_handler)));
        let file_reader_db = database.clone();

        let db_handles = HashMap::new();
        let scan_handles = Arc::new(Mutex::new(HashMap::new()));

        let scanner = AutoScanner::new(namespace, dir, database.clone());
        // registry scanner event handle
        let mut scanner_event_handler = Dispatch::<ScannerRecvArgument>::new();

        let scanner_update_handler_db_arc = database.clone();
        scanner_event_handler.registry(
            scan::PathEvent::NeedClose.as_ref().to_owned(),
            ScannerCloseEvent(scanner_update_handler_db_arc),
        );

        let scanner_needopen_handler_db_arc = database.clone();
        scanner_event_handler.registry(
            scan::PathEvent::NeedOpen.as_ref().to_owned(),
            ScannerOpenEvent(scanner_needopen_handler_db_arc),
        );

        let scanner_needwrite_handler_db_arc = database.clone();
        scanner_event_handler.registry(
            scan::PathEvent::NeedWrite.as_ref().to_owned(),
            ScannerWriteEvent(scanner_needwrite_handler_db_arc),
        );

        Self {
            scanner,
            database,
            file_reader: FileReader::new(file_reader_db),
            _jh: None,
            db_handles,
            scan_handles,
        }
    }

    pub fn start(&mut self) -> Result<()> {
        let cfg = Config::build(Environment::Development)
            .address("0.0.0.0")
            .port(8080)
            .unwrap();

        let database = self.database.clone();
        let scanner_clone = self.scanner.clone();
        let self_scan_handles = self.scan_handles.clone();
        // start auto scanner with a new thread
        let _jh = thread::spawn(move || scanner_clone.start(self_scan_handles));

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

    fn scanner_event_need_close_handle(p: (scan::PathEventInfo, Arc<Mutex<Database>>)) {
        if let Err(e) = p.1.lock().unwrap().delete(p.0.path) {
            eprintln!("Scanner event close error: {:?}", e);
        }
    }
    fn scanner_event_need_open_handle(p: (scan::PathEventInfo, Arc<Mutex<Database>>)) {
        if let Err(e) = p.1.lock().unwrap().put(p.0.to_pod()) {
            eprintln!("Scanner event open error: {:?}", e);
        }
    }
    fn scanner_event_need_write_handle(p: (scan::PathEventInfo, Arc<Mutex<Database>>)) {
        println!("Scanner event write {:?}", p.0)
    }

    // recv db info start collector
    fn database_event_add_pod_handle(&self, pod: Pod) {
        println!("Database add pod {:?}", pod)
    }
    fn database_event_delete_pod_handle(pod: Pod) {
        println!("Database delete pod {:?}", pod)
    }
    fn database_event_update_pod_handle(pod: Pod) {
        println!("Database update pod {:?}", pod)
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
