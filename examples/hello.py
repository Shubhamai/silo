import time
import silo

# [::1]:50051 is the default address of the Silo server
server = silo.Server("localhost:50051", api_key="your_api_key_here")


@server.function()
def hello(name):
    import time
    import socket
    import os
    import sys

    # print(f"Hello, {name}!")
    # time.sleep(4)
    # print(f"Bye, {name}!")

    return f"Hello, {name}!"


@server.entry()
def main():
    start_time = time.time()
    result = hello.remote("Remote")
    print(f"Time taken ms: {round((time.time() - start_time) * 1000, 2)}")
    print(result)


# Run by - silo launch hello.py

# Currently - python examples/silo/cli.py launch examples/hello.py
