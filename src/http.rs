use bytes::Bytes;
use dashmap::DashMap;
use http_body_util::{combinators::BoxBody, BodyExt, Empty, Full};
use hyper::{
    header, server::conn::http1, service::service_fn, Method, Request, Response, StatusCode,
};
use hyper_util::rt::TokioIo;
// use redis::Commands;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;

use crate::grpc::{PythonInput, PythonOutput};

use colored::*;

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
                        // let client = redis::Client::open("redis://0.0.0.0:8080").unwrap();
                        // let mut con = client.get_connection().unwrap();
                        // let _: () = con
                        //     .set(format!("data-func-{}", hostname), data.func)
                        //     .unwrap();
                        // let _: () = con
                        //     .set(format!("data-args-{}", hostname), data.args)
                        //     .unwrap();
                        // let _: () = con
                        //     .set(format!("data-kwargs-{}", hostname), data.kwargs)
                        //     .unwrap();

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

                // let client = redis::Client::open("redis://0.0.0.0:8080").unwrap();
                // let mut con = client.get_connection().unwrap();
                // let func: Vec<u8> = con.get(format!("data-func-{}", hostname)).unwrap();
                // let args: Vec<u8> = con.get(format!("data-args-{}", hostname)).unwrap();
                // let kwargs: Vec<u8> = con.get(format!("data-kwargs-{}", hostname)).unwrap();

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
                // let out = serde_json::from_slice::<PythonOutput>(&whole_body).unwrap();

                // let client = redis::Client::open("redis://0.0.0.0:8080").unwrap();
                // let mut con = client.get_connection().unwrap();
                // let _: () = con.set(format!("output-{}", hostname), out.output).unwrap();

                Ok(Response::builder()
                    .status(StatusCode::OK)
                    .body(empty())
                    .unwrap())
            }
            (&Method::GET, "/output") => {
                let data = self.python_result_data.remove(hostname).unwrap().1;
                // let client = redis::Client::open("redis://0.0.0.0:8080").unwrap();
                // let mut con = client.get_connection().unwrap();
                // let data: Vec<u8> = con.get(format!("output-{}", hostname)).unwrap();

                let data = data.output;

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

    // pub async fn run(self) {
    //     let listener = TcpListener::bind(self.address.clone()).await.unwrap();
    //     println!(
    //         "{}",
    //         format!("HTTP server listening on {}...", self.address).blue()
    //     );

    //     loop {
    //         let (stream, _) = listener.accept().await.unwrap();
    //         let io = TokioIo::new(stream);

    //         let service = service_fn(move |req: Request<hyper::body::Incoming>| {
    //             // let python_input_data = self.python_input_data.clone();
    //             // let python_result_data = self.python_result_data.clone();

    //             async move {
    //                 match self.handle(req).await {
    //                     Ok(response) => Ok::<
    //                         hyper::Response<BoxBody<bytes::Bytes, hyper::Error>>,
    //                         hyper::Error,
    //                     >(response),
    //                     Err(_) => Ok(Response::builder()
    //                         .status(StatusCode::NOT_FOUND)
    //                         .body(empty())
    //                         .unwrap()),
    //                 }
    //             }
    //         });

    //         tokio::task::spawn(async move {
    //             if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
    //                 println!("{}", format!("Error serving connection: {:?}", err).red());
    //             }
    //         });
    //     }
    // }
}

// pub fn http_server(address: String) {
//     std::thread::spawn(move || {
//         // let containers = Arc::new(Mutex::new(HashMap::<String, String>::new()));
//         // let python_input_data = Arc::new(Mutex::new(HashMap::<String, PythonInput>::new()));
//         let python_input_data = DashMap::<std::string::String, PythonInput>::new();
//         // let python_result_data = Arc::new(Mutex::new(HashMap::<String, PythonOutput>::new()));
//         let python_result_data = DashMap::<std::string::String, PythonOutput>::new();

//         let rt = tokio::runtime::Runtime::new().unwrap();

//         rt.block_on(async {
//             let listener = TcpListener::bind(address.clone()).await.unwrap();
//             println!(
//                 "{}",
//                 format!("HTTP server listening on {}...", address).blue()
//             );

//             loop {
//                 let (stream, _) = listener.accept().await.unwrap();
//                 let io = TokioIo::new(stream);

//                 // let python_input_data = python_input_data;
//                 let python_result_data = python_result_data.clone();
//                 // let containers = containers.clone();

//                 let service = service_fn(move |req: Request<hyper::body::Incoming>| {
//                     let python_input_data = python_input_data.clone();
//                     let python_result_data = python_result_data.clone();
//                     // let containers = containers.clone();

//                     async move {
//                         // get hostname header
//                         let headers = req.headers().clone();
//                         let hostname = headers.get("hostname").unwrap().to_str().unwrap();

//                         match (req.method(), req.uri().path()) {
//                             (&Method::PUT, "/data") => {
//                                 let whole_body = req.collect().await?.to_bytes();

//                                 match bincode::decode_from_slice::<PythonInput, _>(
//                                     &whole_body,
//                                     bincode::config::standard(),
//                                 ) {
//                                     Ok(data) => {
//                                         let data = data.0;

//                                         // let mut unck = grpc_request_data.lock().unwrap();
//                                         python_input_data.insert(
//                                             hostname.to_string(),
//                                             PythonInput {
//                                                 func: data.func,
//                                                 args: data.args,
//                                                 kwargs: data.kwargs,
//                                             },
//                                         );

//                                         Ok::<Response<BoxBody<Bytes, hyper::Error>>, hyper::Error>(
//                                             Response::new(empty()),
//                                         )
//                                     }
//                                     Err(_) => Ok(Response::builder()
//                                         .status(StatusCode::BAD_REQUEST)
//                                         .body(empty())
//                                         .unwrap()),
//                                 }
//                             }

//                             (&Method::GET, "/data") => {
//                                 // let mut unck = grpc_request_data.lock().unwrap();
//                                 let data = python_input_data.remove(hostname).unwrap().1;

//                                 Ok(Response::builder()
//                                     .status(StatusCode::OK)
//                                     .header(header::CONTENT_TYPE, "application/json")
//                                     .body(full(
//                                         serde_json::to_string(&HashMap::from([
//                                             ("func".to_string(), data.func.clone()),
//                                             ("args".to_string(), data.args.clone()),
//                                             ("kwargs".to_string(), data.kwargs.clone()),
//                                         ]))
//                                         .unwrap(),
//                                     ))
//                                     .unwrap())
//                             }

//                             (&Method::PUT, "/output") => {
//                                 let whole_body = req.collect().await?.to_bytes();
//                                 // let mut unck = python_result_data.lock().unwrap();
//                                 python_result_data.insert(
//                                     hostname.to_string(),
//                                     serde_json::from_slice(&whole_body).unwrap(),
//                                 );
//                                 // drop(unck);

//                                 Ok(Response::builder()
//                                     .status(StatusCode::NOT_FOUND)
//                                     .body(empty())
//                                     .unwrap())
//                             }
//                             (&Method::GET, "/output") => {
//                                 println!("GET /output {}", hostname);
//                                 // let mut unck = python_result_data.lock().unwrap();
//                                 let data = python_result_data.remove(hostname).unwrap().1;
//                                 // drop(unck);

//                                 Ok(Response::builder()
//                                     .status(StatusCode::OK)
//                                     .header(header::CONTENT_TYPE, "application/json")
//                                     .body(full(
//                                         serde_json::to_string(&HashMap::from([(
//                                             "output".to_string(),
//                                             data.output.clone(),
//                                         )]))
//                                         .unwrap(),
//                                     ))
//                                     .unwrap())
//                             }

//                             _ => Ok(Response::builder()
//                                 .status(StatusCode::NOT_FOUND)
//                                 .body(empty())
//                                 .unwrap()),
//                         }
//                     }
//                 });

//                 tokio::task::spawn(async move {
//                     if let Err(err) = http1::Builder::new().serve_connection(io, service).await {
//                         println!("{}", format!("Error serving connection: {:?}", err).red());
//                     }
//                 });
//             }
//         });
//     });
// }

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
