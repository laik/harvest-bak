use super::Result;
use db::{Database, Pod};
use std::sync::{Arc, Mutex};

use rocket::config::{Config, Environment};
use rocket::State;
use rocket::{delete, get, post, routes};
// use rocket::response::{Failure, status};
use rocket_contrib::json::{Json, JsonValue};
use serde::{Deserialize, Serialize};

use event::EventHandler;
use scan::AutoScanner;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

trait Process {
    fn collect(&self, rx: Receiver<()>);
    fn close(&self, rx: Receiver<()>);
}

pub struct Harvest {
    scanner: AutoScanner,
    database: Arc<Mutex<Database>>,
}

impl Harvest {
    pub fn new(namespace: String, dir: String) -> Self {
        // registry db event handle
        let mut db_event_handler = EventHandler::<Pod>::new();
        db_event_handler.registry(
            db::Event::Add.as_ref().to_owned(),
            Harvest::database_event_add_pod_handle,
        );
        db_event_handler.registry(
            db::Event::Delete.as_ref().to_owned(),
            Harvest::database_event_delete_pod_handle,
        );
        db_event_handler.registry(
            db::Event::Update.as_ref().to_owned(),
            Harvest::database_event_update_pod_handle,
        );
        let database = Arc::new(Mutex::new(Database::new(db_event_handler)));

        // registry scanner event handle
        let mut scanner_event_handler =
            EventHandler::<(scan::PathEventInfo, Arc<Mutex<Database>>)>::new();

        scanner_event_handler.registry(
            scan::PathEvent::NeedClose.as_ref().to_owned(),
            Harvest::scanner_event_need_close_handle,
        );

        scanner_event_handler.registry(
            scan::PathEvent::NeedOpen.as_ref().to_owned(),
            Harvest::scanner_event_need_open_handle,
        );

        scanner_event_handler.registry(
            scan::PathEvent::NeedWrite.as_ref().to_owned(),
            Harvest::scanner_event_need_write_handle,
        );

        let scanner = AutoScanner::new(namespace, dir, database.clone());
        let scanner_clone = scanner.clone();
        // start autoscan with a new thread
        thread::spawn(move || scanner_clone.start(scanner_event_handler));

        Self { scanner, database }
    }

    pub fn start(&self) -> Result<()> {
        let cfg = Config::build(Environment::Development)
            .address("0.0.0.0")
            .port(8080)
            .unwrap();

        let database = self.database.clone();
        rocket::custom(cfg)
            .mount("/", routes![apply_pod, delete_pod, query_pod])
            .register(catchers![not_found])
            .manage(database)
            .launch();

        Ok(())
    }

    fn scanner_event_need_close_handle(p: (scan::PathEventInfo, Arc<Mutex<Database>>)) {
        println!("Scanner event close {:?}", p.0)
    }
    fn scanner_event_need_open_handle(p: (scan::PathEventInfo, Arc<Mutex<Database>>)) {
        println!("Scanner event open {:?}", p.0)
    }
    fn scanner_event_need_write_handle(p: (scan::PathEventInfo, Arc<Mutex<Database>>)) {
        println!("Scanner event write {:?}", p.0)
    }
    fn database_event_add_pod_handle(pod: Pod) {
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
            if let Err(e) = db.put(
                "".to_owned(),
                Pod {
                    uuid: "".to_string(),
                    offset: 0,
                    inode: 0,
                    namespace: req.0.namespace,
                    pod_name: req.0.pod,
                    container_name: req.0.container,
                },
            ) {
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
