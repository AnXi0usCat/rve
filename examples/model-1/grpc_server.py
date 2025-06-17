import argparse
import asyncio
import grpc
import json
import pandas as pd
import service_pb2
import service_pb2_grpc

parser = argparse.ArgumentParser()
parser.add_argument("--port", type=int)

class ProxyService(service_pb2_grpc.ProxyServiceServicer):
    async def Predict(self, request, context):
        print(f"Received request: {request.json_request}")
        input_data = json.loads(request.json_request)
        output_data = {
            "message": f"Installed pandas version == {pd.__version__}"
        }
        response_json = json.dumps(output_data)
        return service_pb2.PredictResponse(json_response=response_json)

async def serve():
    args = parser.parse_args()
    server = grpc.aio.server()
    service_pb2_grpc.add_ProxyServiceServicer_to_server(ProxyService(), server)
    server.add_insecure_port(f"[::1]:{args.port}")
    print(f"Starting async gRPC server on port {args.port}...")
    await server.start()

    try:
        await server.wait_for_termination()
    except KeyboardInterrupt:
        print("Shutting down gRPC server...")
        await server.stop(grace=5)  # Graceful shutdown


if __name__ == "__main__":
    asyncio.run(serve())
