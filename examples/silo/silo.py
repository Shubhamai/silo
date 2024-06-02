import cloudpickle
import pickle
from silo_pb2 import GetPackageRequest
from silo_pb2_grpc import SiloStub
import grpc
import requests
import concurrent.futures
import numpy as np


class Server:
    def __init__(self, url, api_key=None):
        self.api_key = api_key
        self.web_url = "http://localhost:3000"

        # if url is string, then it is the address of the Silo server
        if isinstance(url, str):
            channel = grpc.insecure_channel(url)
            self.client = SiloStub(channel)
        elif isinstance(url, list):
            # NOTE: If multiple clients, then the first client is the default client
            self.client = SiloStub(grpc.insecure_channel(url[0]))
            self.clients = [SiloStub(grpc.insecure_channel(address)) for address in url]
        else:
            raise ValueError("Invalid address, must be string or list")

    def function(self):
        def decorator(func):
            return RemoteFunction(self, func)

        return decorator

    def entry(self):
        def decorator(func):
            def wrapper(*args, **kwargs):
                return func(*args, **kwargs)

            return wrapper

        return decorator

    def get_func(self, cid, key):

        data = requests.get(f"https://gateway.lighthouse.storage/ipfs/{cid}").text

        response = requests.patch(
            f"{self.web_url}/api/upload", json={"data": data, "key": key}
        )

        return RemoteFunction(self, pickle.loads(bytes(response.json()["func"])))


class RemoteFunction:
    def __init__(self, server, func):
        self.server = server
        self.func = func

    def _make_request(self, endpoint, request=None):
        # if multiple clients, then select a random client
        if hasattr(self.server, "clients"):
            client = np.random.choice(self.server.clients)
        else:
            client = self.server.client

        response = client.GetPackage(request)

        return response

    def _upload_data(self, data):
        response = requests.post(f"{self.server.web_url}/api/upload", json=data)
        return response.json()

    def remote(self, *args, **kwargs):
        # Verify computation with local computation randomly
        verify_compute = np.random.rand() > 0.9

        data = {
            "func": list(cloudpickle.dumps(self.func)),
            "args": list(cloudpickle.dumps(args)),
            "kwargs": list(cloudpickle.dumps(kwargs)),
        }

        response = self._upload_data(data)
        # response = {
        #     "hash": "QmQd9f1YUFPuwYNRsBqhjWiwkAXFzW5D33CrMkJictfXKo",
        #     "key": "62eea30efdcd20b5a770c5df5a0cd86819b5ce5cf426793829c33f2bb77d3767",
        # }

        request = GetPackageRequest()
        request.cid = response["hash"]
        request.key = response["key"]

        # print(f"CID: {request.cid} Key: {request.key}")

        output = self._make_request("execute", request)

        unpickled_output = pickle.loads(output.output)

        if verify_compute:
            assert unpickled_output == self.local(
                *args, **kwargs
            ), "Computation from server is incorrect"

        return unpickled_output

    def map(self, inputs):
        with concurrent.futures.ThreadPoolExecutor() as executor:
            result = list(
                executor.map(
                    self.remote,
                    inputs,
                )
            )

        return result

    def local(self, *args, **kwargs):
        function_code = cloudpickle.dumps(self.func)

        return pickle.loads(function_code)(*args, **kwargs)
