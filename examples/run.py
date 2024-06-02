import silo

server = silo.Server(["localhost:50051", "localhost:50052"])
# server = silo.Server("localhost:50051")

hello = server.get_func(
            "QmWfUGByP4yzVQsB3U2AJvcNMyp7TXrmt6MapdCzgiqdhC", # IPFS CID
            "bd142c6df1cddd01fb540bb04d210f8808fb8e9443a390859d2fcddaa9e2586e" # Decrypt Key
            )

result = hello.map(["EthGlobal"] * 10)
print(result)
