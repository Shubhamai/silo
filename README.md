## Silo

Silo is my attempt to learn how Modal Labs works. I learn a lot from these two blogs,

### How to try!

There are several different parts to the application but runnning a single command `cargo run --release` should start everything locally.

Once the server is running, simply run

```
python silo/cli.py launch examples/hello.py
```

### Under the hood

`cargo run --release`

When you ran the above command, the following things happened:

1. The gRPC server started listening for incoming requests from clients containing pickled python code.
2. An HTTP server was started on a new thread to facititate communication between the containers.

3. A FUSE filesystem was mounted on the host machine to get files from the indexer.
4. An indexer was started to as TCP server to serve container images files. It can also index new container images via CLI.

When you ran the `python silo/cli.py launch examples/hello.py` command, the following things happened:

1. The python code was pickled and sent to the gRPC server.
2. The gRPC server sends the data to the HTTP server which saves the data to a file, and forwards the file to the container when it is launched.

3. A Fuse filesystem was mounted on the host machine to get files from the container.
4. The container is launched using podman mounted with the FUSE filesystem.

5. The container runs a python code which asks the python code to be executed from the HTTP server.
6. The python code is executed and the output is sent back to the http server, onced container has exited, the gRPC server then ask HTTP server for output and finally sends it to the client.


# gRPC Server
[gRPC](src/grpc.rs)





# Content Indexer

The job if the indexer is to index the contents of Podman container images, store the indexed data, and serve it via a TCP server. The system is designed to provide quick access to their file structure and content.

## Table of Contents

1. [Overview](#overview)
2. [Components](#components)
3. [Installation](#installation)
4. [Usage](#usage)
5. [Architecture](#architecture)
6. [Key Features](#key-features)
7. [Performance Considerations](#performance-considerations)

## Overview

The Content Indexer works the following way:

1. Indexing Podman container images
2. Storing indexed data in a SQLite database
3. Serving indexed data via a TCP server
4. Listing indexed images

## Components

The project consists of several key components:

1. **ContentIndexer**: The core indexing logic
2. **AppState**: Manages the application state and database operations
3. **TCP Server**: Serves indexed data to clients
4. **CLI Interface**: Handles user commands for indexing and listing images

## Installation

To install and run the Content Indexer, follow these steps:

1. Ensure you have Rust and Cargo installed on your system.
2. Clone the repository.
3. Navigate to the project directory.
4. Run `cargo build --release` to compile the project.

## Usage

The Content Indexer can be used via command-line interface:

```
content_indexer [OPTIONS] [COMMAND]
OPTIONS:
--host <HOST> [default: 127.0.0.1]
-p, --port <PORT> [default: 8080]
-s, --storage <STORAGE> [default: "./content"]
-d, --db <DB> [default: "indexer.db"]
COMMANDS:
index <IMAGE_NAME> Index a podman image
list List indexed podman images
help Print this message or the help of the given subcommand(s)
```

### Indexing an Image

To index a Podman image:

```
content_indexer index <IMAGE_NAME>
```

### Listing Indexed Images

To list all indexed images:

```
content_indexer list
```

## Architecture

The Content Indexer follows a modular architecture:

1. **Main Module** (`src/indexer/main.rs`):

   - Initializes the application
   - Parses command-line arguments
   - Starts the TCP server
   - Handles CLI commands

2. **Indexer Module** (`src/indexer/indexer.rs`):

   - Implements the `ContentIndexer` struct
   - Handles the core indexing logic

3. **Commands Module** (`src/indexer/commands.rs`):

   - Implements the `index_image` and `list_images` functions
   - Manages the process of pulling, running, and mounting containers

4. **Server Module** (`src/indexer/server.rs`):

   - Implements the TCP server
   - Handles client connections and requests

5. **Database Module** (`src/indexer/database.rs`):
   - Manages the SQLite database operations
   - Implements the `AppState` struct for managing application state

## Key Features

1. **Efficient Indexing**: The system uses a content-addressable storage approach, storing file contents based on their SHA256 hash.

2. **Caching**: The TCP server implements a cache to improve performance when serving frequently requested files.

3. **Concurrent Processing**: The indexer uses atomic operations to track progress, allowing for potential parallelization in future improvements.

4. **Flexible Storage**: The indexed data is stored in a SQLite database, allowing for easy querying and management.

5. **TCP Server**: Provides a simple interface for clients to request indexed data.

## Performance Considerations

The Content Indexer is designed with performance in mind:

1. **Content Deduplication**: By using content-addressable storage, duplicate files across different images only need to be stored once.

2. **Caching**: The TCP server implements a cache to reduce disk I/O for frequently accessed files.

3. **Efficient Querying**: The use of a SQLite database allows for quick retrieval of indexed data.

4. **Progress Tracking**: The system tracks the number of processed files, allowing for progress reporting during indexing.

For more detailed information on specific components, refer to the inline documentation in the source code.
