import time
import silo

# [::1]:50051 is the default address of the Silo server
# server = silo.Server("146.190.78.181:50051", api_key="your_api_key_here")
server = silo.Server("0.0.0.0:50051", api_key="your_api_key_here")


@server.function()
def hello(name):

    print(f"Hello, {name}!")
    # time.sleep(10)
    print(f"Bye, {name}!")

    return f"Hello, {name}!"


result = hello.remote(name="Remote")
# result = hello.map(f"Remote {i}" for i in range(10))

print(result)


# @server.entry()
# def main():
#     start_time = time.time()
#     try:

#         # Example of launching a single container
#         # result = hello.remote("Remote")

#         # Example of launching multiple containers
#         # result = hello.map(["Remote"] * 20)

#         # Example of launching a container with a saved function data CID and key
#         hello = server.get_func("hello")
#         result = hello(name="Remote")

#         print(f"Time taken ms: {round((time.time() - start_time) * 1000, 2)}")
#         print(result)
#     except Exception as e:
#         print(e)
#         time.sleep(50)


# Run by - silo launch hello.py

# curl --proto '=https' --tlsv1.2 -LsSf https://github.com/shubhamai/silo/releases/download/v0.1.0/silo_installer.sh | sh

# clear && cargo build --release && time sudo ./target/release/silo facility
# Currently - python examples/silo/cli.py launch examples/hello.py
# python examples/silo/cli.py build examples/hello.py
# python -m grpc_tools.protoc -I../../common/protobufs --python_out=. --pyi_out=. --grpc_python_out=. ../../common/protobufs/silo.proto
# python -m grpc_tools.protoc -I./common/protobufs/ --python_out=./examples/silo --grpc_python_out=./examples/silo silo.proto
