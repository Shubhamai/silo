import cloudpickle
import pickle
from silo_pb2 import GetPackageRequest
from silo_pb2_grpc import SiloStub
import grpc
import requests
import concurrent.futures


class Server:
    def __init__(self, url, api_key=None):
        self.api_key = api_key
        self.web_url = "http://localhost:3000"

        channel = grpc.insecure_channel(url)
        self.client = SiloStub(channel)

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

    def launch(self, cid, key):
        def get_input(*args, **kwargs):
            request = GetPackageRequest()
            request.cid = cid
            request.key = key

            output = self.client.GetPackage(request)
            return pickle.loads(output.output)

        return get_input


class RemoteFunction:
    def __init__(self, server, func):
        self.server = server
        self.func = func

    def _make_request(self, endpoint, request=None):
        headers = {}
        if self.server.api_key:
            headers["X-API-Key"] = self.server.api_key

        response = self.server.client.GetPackage(request)

        return response

    def _upload_data(self, data):
        response = requests.post(f"{self.server.web_url}/api/upload", json=data)
        return response.json()

    def remote(self, *args, **kwargs):
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

        print(f"CID: {request.cid} Key: {request.key}")

        output = self._make_request("execute", request)

        return pickle.loads(output.output)

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
