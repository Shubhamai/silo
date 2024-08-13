import silo

server = silo.Server("0.0.0.0:50051", api_key="your_api_key_here")


@server.function(image="python:3.10")
def hello(name):

    print(f"Hello, {name}!")

    return f"Bye, {name}!"


result = hello.remote(name="Remote")
# result = hello.map(f"Remote {i}" for i in range(10))

print(result)
