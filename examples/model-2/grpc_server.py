import argparse
import grpc
from concurrent import futures
import json
import pandas as pd
import service_pb2
import service_pb2_grpc

parser = argparse.ArgumentParser()
parser.add_argument("--port", type=int)


class ProxyService(service_pb2_grpc.ProxyServiceServicer):
    def Predict(self, request, context):
        print(f"Received request: {request.json_request}")

        # Example logic: parse input JSON and respond with JSON
        input_data = json.loads(request.json_request)
        output_data = {
            "received": input_data,
            "message": f"Installed pandas version == {pd.__version__}"
        }

        response_json = json.dumps(output_data)
        return service_pb2.PredictResponse(json_response=response_json)

def serve():
    args = parser.parse_args()
    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    service_pb2_grpc.add_ProxyServiceServicer_to_server(ProxyService(), server)
    server.add_insecure_port(f"[::1]:{args.port}")
    print(f"Starting gRPC server on port {args.port}...")
    server.start()
    server.wait_for_termination()

if __name__ == "__main__":
    serve()

