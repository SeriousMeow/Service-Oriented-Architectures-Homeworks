FROM python:3.13-slim

WORKDIR /app

ENV PYTHONDONTWRITEBYTECODE=1 \
    PYTHONUNBUFFERED=1

COPY ./tests/requirements.txt /app/requirements.txt
RUN pip install --no-cache-dir -r /app/requirements.txt

# Copy tests and proto sources.
COPY ./tests /app/tests
COPY ./proto /app/proto

# Generate gRPC python stubs.
RUN python -c "import grpc_tools; print(grpc_tools.__path__[0] + '/_proto')" > /tmp/grpc_tools_proto_path \
    && GRPC_TOOLS_PROTO_PATH="$(cat /tmp/grpc_tools_proto_path)" \
    && mkdir -p /app/generated \
    && python -m grpc_tools.protoc \
      -I/app/proto \
      -I"$GRPC_TOOLS_PROTO_PATH" \
      --python_out=/app/generated \
      --grpc_python_out=/app/generated \
      /app/proto/flight/v1/flight.proto

ENV PYTHONPATH="/app:/app/generated"

CMD ["pytest", "-q", "/app/tests"]

