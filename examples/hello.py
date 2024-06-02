import time
import silo

# server = silo.Server(["localhost:50051", "localhost:50052"])
server = silo.Server("localhost:50051")


@server.function()
def hello(name):
    # import numpy

    # print(f"Hello, {name}!")
    # time.sleep(20)
    # print(f"Bye, {name}!")

    return f"Hello, {name}!"


# result = hello.remote("World")
# result = hello.map(["Remote"] * 10)
# print(result)

@server.entry()
def main():
    start_time = time.time()
    try:
        # Example of launching a single container
        # result = hello.remote("World")

        # Example of launching multiple containers in multiple compute providers
        result = hello.map(f"World {i}" for i in range(10))

        # Example of launching a container with a saved function data CID and key
        # hello = server.get_func(
        #     "QmWfUGByP4yzVQsB3U2AJvcNMyp7TXrmt6MapdCzgiqdhC",
        #     "bd142c6df1cddd01fb540bb04d210f8808fb8e9443a390859d2fcddaa9e2586e",
        # )
        # result = hello.map(["sdfdsfs"] * 10)
        # result = hello.remote("Remote")
        # result = hello.local("Local")

        print(f"Time taken ms: {round((time.time() - start_time) * 1000, 2)}")
        print(result)
    except Exception as e:
        print(e)
        time.sleep(50)


# Run by - silo launch hello.py

# clear && cargo build --release && time sudo ./target/release/silo facility
# Currently - python examples/silo/cli.py launch examples/hello.py
# python examples/silo/cli.py build examples/hello.py
# sudo ./silo facility --grpc-port 50052 --http-port 8001