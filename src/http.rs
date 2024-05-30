use bincode::{Decode, Encode};
use bytes::Bytes;
use dashmap::{DashMap, DashSet};
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{header, Method, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Encode, Decode, PartialEq, Debug, Clone, Deserialize, Serialize, Eq, Hash)]
pub struct PythonInput {
    pub request_id: RequestID,
    pub func: Vec<u8>,
    pub args: Vec<u8>,
    pub kwargs: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PythonOutput {
    // pub request_id: RequestID,
    pub output: Vec<u8>,
}

type RequestID = i32;
type UserID = String;
type IsFree = bool;
type ContainerID = String;
type InputID = u32;
pub struct HttpServer {
    pub address: String,
    pub containers: DashMap<ContainerID, IsFree>,
    pub tasks: DashSet<PythonInput>,
    // pub inputs: DashMap<InputID, PythonInput>,
    pub results: DashMap<RequestID, PythonOutput>,
}

impl HttpServer {
    pub async fn handle(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        // get hostname header
        let headers = req.headers().clone();
        // let hostname = headers.get("hostname").unwrap().to_str().unwrap();

        // /data
        // PUT - add data from the gRPC server for the container
        // GET - data requested by python in container to run

        // /output
        // PUT - add output from the container run
        // GET - output requested by the gRPC server

        // /task
        // PUT - add task with respective data to run
        // GET - task along with the data requested by the container

        // /containers
        // PUT - add container and its status
        // GET - get all containers and their status

        match (req.method(), req.uri().path()) {
            (&Method::PUT, "/tasks") => {
                let whole_body = req.collect().await?.to_bytes();

                match bincode::decode_from_slice::<PythonInput, _>(
                    &whole_body,
                    bincode::config::standard(),
                ) {
                    Ok(data) => {
                        self.tasks.insert(data.0);
                        // print all containers
                        for container in self.containers.iter() {
                            println!("{:?}", container.key());
                        }

                        // if no container is free, then send info to launch a new container
                        if self.containers.iter().all(|x| *x.value() == false) {
                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .body(full("launch".to_string()))
                                .unwrap())
                        } else {
                            // will send data to the container when it is asked
                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .body(full("nothing".to_string()))
                                .unwrap())
                        }
                    }
                    Err(_) => Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(empty())
                        .unwrap()),
                }
            }

            (&Method::GET, "/tasks") => {
                let res: (
                    Option<PythonInput>,
                    Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error>,
                ) = match self.tasks.iter().next() {
                    Some(task) => {
                        let data = task.clone();

                        (
                            Some(data.clone()),
                            Ok(Response::builder()
                                .status(StatusCode::OK)
                                .header(header::CONTENT_TYPE, "application/json")
                                .body(full(serde_json::to_string(&data).unwrap()))
                                .unwrap()),
                        )
                    }
                    None => (
                        None,
                        Ok(Response::builder()
                            .status(StatusCode::NO_CONTENT)
                            .body(empty())
                            .unwrap()),
                    ),
                };
                if let Some(task) = res.0 {
                    self.tasks.remove(&task);
                }

                res.1
            }
            (&Method::PUT, "/output") => {
                let whole_body = req.collect().await?.to_bytes();

                let output: PythonOutput = serde_json::from_slice(&whole_body).unwrap();
                let request_id = headers
                    .get("request_id")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse()
                    .unwrap();

                self.results.insert(request_id, output);

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(empty())
                    .unwrap())
            }
            (&Method::GET, "/output") => {
                let request_id = headers
                    .get("request_id")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse()
                    .unwrap();

                let data = self.results.get(&request_id).unwrap().clone();

                self.results.remove(&request_id);

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(full(serde_json::to_string(&data).unwrap()))
                    .unwrap())
            }
            (&Method::PUT, "/containers") => {
                // let whole_body = req.collect().await?.to_bytes();
                let container_id = headers
                    .get("container_id")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse()
                    .unwrap();
                let is_free = headers
                    .get("is_free")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse()
                    .unwrap();

                self.containers.insert(container_id, is_free);

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(empty())
                    .unwrap())
            }
            (&Method::DELETE, "/containers") => {
                let container_id: String = headers
                    .get("container_id")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .parse()
                    .unwrap();

                self.containers.remove(&container_id);

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(empty())
                    .unwrap())
            }
            _ => Ok(Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(empty())
                .unwrap()),
        }
    }
}

fn full<T: Into<Bytes>>(chunk: T) -> http_body_util::combinators::BoxBody<Bytes, hyper::Error> {
    Full::new(chunk.into())
        .map_err(|never| match never {})
        .boxed()
}

fn empty() -> BoxBody<Bytes, hyper::Error> {
    Empty::<Bytes>::new()
        .map_err(|never| match never {})
        .boxed()
}
