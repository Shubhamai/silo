use bytes::Bytes;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{
    header, server::conn::http1, service::service_fn, Method, Request, Response, StatusCode,
};
use hyper_util::rt::TokioIo;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;

use crate::grpc::{PythonInput, PythonOutput};

use colored::*;

pub fn http_server(address: String) {
    std::thread::spawn(move || {
        // let containers = Arc::new(Mutex::new(HashMap::<String, String>::new()));
        let python_input_data = Arc::new(Mutex::new(HashMap::<String, PythonInput>::new()));
        let python_result_data = Arc::new(Mutex::new(HashMap::<String, PythonOutput>::new()));

        let rt = tokio::runtime::Runtime::new().unwrap();

        rt.block_on(async {
            let listener = TcpListener::bind(address.clone()).await.unwrap();
            println!(
                "{}",
                format!("HTTP server listening on {}...", address).blue()
            );

            loop {
                let (stream, _) = listener.accept().await.unwrap();
                let io = TokioIo::new(stream);

                let grpc_request_data = python_input_data.clone();
                let python_result_data = python_result_data.clone();
                // let containers = containers.clone();

                let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                    let grpc_request_data = grpc_request_data.clone();
                    let python_result_data = python_result_data.clone();
                    // let containers = containers.clone();

                    async move {
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

                                        let mut unck = grpc_request_data.lock().unwrap();
                                        unck.insert(
                                            hostname.to_string(),
                                            PythonInput {
                                                func: data.func,
                                                args: data.args,
                                                kwargs: data.kwargs,
                                            },
                                        );

                                        Ok::<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error>(
                                            Response::new(empty()),
                                        )
                                    }
                                    Err(_) => Ok(Response::builder()
                                        .status(StatusCode::BAD_REQUEST)
                                        .body(empty())
                                        .unwrap()),
                                }
                            }

                            (&Method::GET, "/data") => {
                                let mut unck = grpc_request_data.lock().unwrap();

                                println!("{:?}", hostname);
                                let data = unck.remove(hostname).unwrap();

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
                                let mut unck = python_result_data.lock().unwrap();
                                unck.insert(
                                    hostname.to_string(),
                                    serde_json::from_slice(&whole_body).unwrap(),
                                );

                                Ok(Response::builder()
                                    .status(StatusCode::NOT_FOUND)
                                    .body(empty())
                                    .unwrap())
                            }
                            (&Method::GET, "/output") => {
                                let mut unck = python_result_data.lock().unwrap();
                                let data = unck.remove(hostname).unwrap();

                                Ok(Response::builder()
                                    .status(StatusCode::OK)
                                    .header(header::CONTENT_TYPE, "application/json")
                                    .body(full(
                                        serde_json::to_string(&HashMap::from([(
                                            "output".to_string(),
                                            data.output.clone(),
                                        )]))
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
                });

                tokio::task::spawn(async move {
                    if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
                        println!("{}", format!("Error serving connection: {:?}", err).red());
                    }
                });
            }
        });
    });
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
