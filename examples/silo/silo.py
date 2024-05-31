import cloudpickle
import pickle
from silo_pb2 import GetPackageRequest
from silo_pb2_grpc import SiloStub
import grpc
import requests


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
        # response = {"data": {"Hash": "QmQEyPkmVffrSe8WTNhjLdo58BFyDvBDq6HLRwfwdeaYwm"}}

        request = GetPackageRequest()
        request.cid = response["data"]["Hash"]

        output = self._make_request("execute", request)

        return pickle.loads(output.output)

    def local(self, *args, **kwargs):
        function_code = cloudpickle.dumps(self.func)

        return pickle.loads(function_code)(*args, **kwargs)
