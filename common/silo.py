import requests
import cloudpickle
import time
import socket
import base64
import os

start = time.perf_counter()

host_link = os.environ.get("HOST_LINK")
task_id = os.environ.get("TASK_ID")

url = f"{host_link}/api/tasks/{task_id}"
response = requests.get(url)

if response.status_code == 200:
    task = response.json()
    
    func = cloudpickle.loads(base64.b64decode(task["func"]))
    args = cloudpickle.loads(base64.b64decode(task["args"]))
    kwargs = cloudpickle.loads(base64.b64decode(task["kwargs"]))
    
    output = func(*args, **kwargs)

    result = cloudpickle.dumps(output)

    requests.post(f"{host_link}/api/results/{task_id}", data=base64.b64encode(result))

    end = time.perf_counter() - start
    print(f"Python time taken: {end * 1000:.2f}ms")
else:
    print("Failed with status code:", response.status_code)