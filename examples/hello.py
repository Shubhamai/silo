import time
import silo
import concurrent.futures

# [::1]:50051 is the default address of the Silo server
server = silo.Server(["localhost:50051", "localhost:50052"], api_key="your_api_key_here")


@server.function()
def hello(name):

    # print(f"Hello, {name}!")
    # time.sleep(20)
    # print(f"Bye, {name}!")

    return f"Hello, {name}!"


@server.entry()
def main():
    start_time = time.time()

    # run the servers on 3 different digitalocean machines, use round robin to distribute the requests
    try:
        # result = hello.remote("Remote")
        with concurrent.futures.ThreadPoolExecutor() as executor:
            result = list(
                executor.map(
                    hello.remote,
                    ["remote"] * 30,
                )
            )
        print(f"Time taken ms: {round((time.time() - start_time) * 1000, 2)}")
        print(result)
    except Exception as e:
        print(e)
        time.sleep(50)


# Run by - silo launch hello.py

# clear && cargo build --release && time sudo ./target/release/silo facility
# Currently - python examples/silo/cli.py launch examples/hello.py
# python examples/silo/cli.py build examples/hello.py
