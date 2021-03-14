use super::{rules_json, set_rule, Rule};
use rocket::{get, post};
use rocket_contrib::json::{Json, JsonValue};
use serde::{Deserialize, Serialize};
use sse_client::EventSource;

const RUN: &'static str = "run";
const STOP: &'static str = "stop";

pub(crate) fn recv_rule(addr: &str, node_name: &str) {
    let event_sources = match EventSource::new(addr) {
        Ok(it) => it,
        Err(e) => {
            eprintln!("{:?}", e);
            return;
        }
    };

    println!("🚀 start watch to api server");

    for event in event_sources.receiver().iter() {
        println!("recv event {:?}", event);
        let request = match serde_json::from_str::<ApiServerRequest>(&event.data) {
            Ok(it) => it,
            Err(e) => {
                eprintln!(
                    "recv event parse json error or connect to api server error: {:?}",
                    e
                );
                continue;
            }
        };
        if !request.has_node_events(&node_name) {
            continue;
        }

        for pod in request.pods.iter() {
            if request.op == RUN {
                set_rule(
                    pod.pod.into(),
                    Rule {
                        upload: true,
                        ..Default::default()
                    },
                );
                db::pod_upload_start(request.ns, pod.pod);
            } else if request.op == STOP {
                db::pod_upload_stop(request.ns, pod.pod.into());
            } else {
                println!("api server event source send unknow event {:?}", request)
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Pod<'a> {
    node: &'a str,
    pod: &'a str,
    ips: Vec<&'a str>,
    offset: i64,
}

//{"op":"add","ns":"default","service_name":"example_service","pods":[{"node":"node1","pod":"example","ips":["127.0.0.1"],"offset":0}]}
#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct ApiServerRequest<'a> {
    op: &'a str,
    ns: &'a str,
    service_name: &'a str,
    pods: Vec<Pod<'a>>,
}

impl<'a> ApiServerRequest<'a> {
    pub fn has_node_events(&self, node_name: &str) -> bool {
        for pod in self.pods.iter() {
            if pod.node == node_name {
                return true;
            }
        }
        false
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Request {
    namespace: String,
    pod: String,
    filter: String,
    output: String,
    upload: bool,
}

#[get("/rules")]
pub(crate) fn query_rules() -> JsonValue {
    json!(rules_json())
}

// /pod/collect list ns.pod start collect to output
#[post("/pod", format = "json", data = "<req>")]
pub(crate) fn post_pod(req: Json<Request>) -> JsonValue {
    if req.0.namespace == "" || req.0.pod == "" {
        return json!({
            "status": "error",
            "reason": format!("namespace {} or pod {} maybe is empty",req.namespace,req.pod),
        });
    }

    for (_, pod) in db::get_slice_with_ns_pod(&req.0.namespace, &req.0.pod).iter() {
        let mut pod = pod.to_owned();
        pod.is_upload = req.0.upload;
        pod.filter = req.0.filter.clone();
        pod.output = req.0.output.clone();
        db::apply(&pod);
    }

    json!({"status":"ok"})
}

#[get("/pod")]
pub(crate) fn query_pod() -> JsonValue {
    json!(db::all_to_json())
}

#[catch(404)]
pub(crate) fn not_found() -> JsonValue {
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
