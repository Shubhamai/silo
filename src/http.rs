use bytes::Bytes;
use dashmap::DashMap;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{header, Method, Request, Response, StatusCode};
use std::collections::HashMap;

use crate::grpc::{PythonInput, PythonOutput};

pub struct HttpServer {
    pub address: String,
    pub python_input_data: DashMap<std::string::String, PythonInput>,
    pub python_result_data: DashMap<std::string::String, PythonOutput>,
}

impl HttpServer {
    pub async fn handle(
        &self,
        req: Request<hyper::body::Incoming>,
    ) -> Result<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error> {
        // get hostname header
        let headers = req.headers().clone();
        let hostname = headers.get("hostname").unwrap().to_str().unwrap();

        match (req.method(), req.uri().path()) {
            (&Method::PUT, "/data") => {
                let whole_body = req.collect().await?.to_bytes();

                match bincode::decode_from_slice::<PythonInput, _>(
                    &whole_body,
                    bincode::config::standard(),
                ) {
                    Ok(data) => {
                        let data = data.0;

                        // let mut unck = grpc_request_data.lock().unwrap();
                        self.python_input_data.insert(
                            hostname.to_string(),
                            PythonInput {
                                func: data.func,
                                args: data.args,
                                kwargs: data.kwargs,
                            },
                        );

                        Ok::<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error>(Response::new(
                            empty(),
                        ))
                    }
                    Err(_) => Ok(Response::builder()
                        .status(StatusCode::BAD_REQUEST)
                        .body(empty())
                        .unwrap()),
                }
            }

            (&Method::GET, "/data") => {
                // let mut unck = grpc_request_data.lock().unwrap();
                let data = self.python_input_data.remove(hostname).unwrap().1;

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(full(
                        serde_json::to_string(&HashMap::from([
                            ("func".to_string(), data.func.clone()),
                            ("args".to_string(), data.args.clone()),
                            ("kwargs".to_string(), data.kwargs.clone()),
                        ]))
                        .unwrap(),
                    ))
                    .unwrap())
            }

            (&Method::PUT, "/output") => {
                let whole_body = req.collect().await?.to_bytes();

                self.python_result_data.insert(
                    hostname.to_string(),
                    serde_json::from_slice(&whole_body).unwrap(),
                );

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(empty())
                    .unwrap())
            }
            (&Method::GET, "/output") => {
                let data = self.python_result_data.remove(hostname).unwrap().1.output;

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(full(
                        serde_json::to_string(&HashMap::from([("output".to_string(), data)]))
                            .unwrap(),
                    ))
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
