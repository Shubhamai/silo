import base64
import cloudpickle
import pickle
from silo_pb2 import GetPackageRequest
from silo_pb2_grpc import SiloStub
import grpc
import inspect

import concurrent.futures


class Server:
    def __init__(self, url, api_key=None):
        self.api_key = api_key

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

    def get_func(self, cid, key):

        print("TODO: Implement get_func")

        # return RemoteFunction(self, pickle.loads(bytes(response.json()["func"])))

        pass


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

    def remote(self, *args, **kwargs):

        request = GetPackageRequest()

        request.func_str = inspect.getsource(self.func)
        request.func = base64.b64encode(cloudpickle.dumps(self.func)).decode("utf-8")
        request.args = base64.b64encode(cloudpickle.dumps(args)).decode("utf-8")
        request.kwargs = base64.b64encode(cloudpickle.dumps(kwargs)).decode("utf-8")

        response = self._make_request("execute", request)

        return pickle.loads(base64.b64decode(response.output))

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
