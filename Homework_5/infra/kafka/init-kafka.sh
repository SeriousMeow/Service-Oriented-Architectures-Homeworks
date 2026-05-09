#!/usr/bin/env bash

set -euo pipefail

kafka-topics --bootstrap-server "${KAFKA_BOOTSTRAP}" \
  --create --if-not-exists \
  --topic "${TOPIC_NAME}" \
  --partitions "${PARTITIONS}" \
  --replication-factor "${REPLICATION_FACTOR}" \
  --config "min.insync.replicas=${KAFKA_TOPIC_MIN_ISR}"

SCHEMA_JSON=$(jq -c . < "${SCHEMA_FILE}")
PAYLOAD=$(jq -n --arg s "${SCHEMA_JSON}" '{schema: $s}')

curl -sS -o /tmp/sr-register.json -w "%{http_code}" \
  -X POST \
  -H "Content-Type: application/vnd.schemaregistry.v1+json" \
  --data-binary "${PAYLOAD}" \
  "${SCHEMA_REGISTRY_URL}/subjects/${SUBJECT}/versions"

