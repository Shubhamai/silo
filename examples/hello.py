import silo

# server = silo.Server("localhost:50051")
server = silo.Server(["localhost:50051", "localhost:50052"])

@server.function()
def hello(name):
    return f"GM, {name}!"

# result = hello.remote("World")
result = hello.map(f"Remote {i}" for i in range(10))

print(result)
