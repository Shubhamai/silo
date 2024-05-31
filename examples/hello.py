import time
import silo

# [::1]:50051 is the default address of the Silo server
server = silo.Server("localhost:50051", api_key="your_api_key_here")


@server.function()
def hello(name):

    # print(f"Hello, {name}!")
    # time.sleep(20)
    # print(f"Bye, {name}!")

    return f"Hello, {name}!"


@server.entry()
def main():
    start_time = time.time()
    try:
        # Example of launching a single container
        # result = hello.remote("Remote")

        # Example of launching multiple containers
        # result = hello.map(["Remote"] * 10)

        # Example of launching a container with a saved function data CID and key
        hello = server.launch(
            "QmZb1sXB8hbdha3bKdTdHQmGwx5fWmYcVy5frpk5WK8KkM",
            "01425e7c585bf1528477ec6e2839e0c0b760481e97c18ae4f0e240e6ef7e7581",
        )
        result = hello(name="Remote")

        print(f"Time taken ms: {round((time.time() - start_time) * 1000, 2)}")
        print(result)
    except Exception as e:
        print(e)
        time.sleep(50)


# Run by - silo launch hello.py

# clear && cargo build --release && time sudo ./target/release/silo facility
# Currently - python examples/silo/cli.py launch examples/hello.py
# python examples/silo/cli.py build examples/hello.py
