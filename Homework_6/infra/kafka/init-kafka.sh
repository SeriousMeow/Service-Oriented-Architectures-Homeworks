#!/usr/bin/env bash

set -euo pipefail

kafka-topics --bootstrap-server "${KAFKA_BOOTSTRAP}" \
  --create --if-not-exists \
  --topic "${TOPIC_NAME}" \
  --partitions "${PARTITIONS}" \
  --replication-factor "${REPLICATION_FACTOR}" \
  --config "min.insync.replicas=${KAFKA_TOPIC_MIN_ISR}"

kafka-topics --bootstrap-server "${KAFKA_BOOTSTRAP}" \
  --create --if-not-exists \
  --topic "${DLQ_TOPIC_NAME}" \
  --partitions "${PARTITIONS}" \
  --replication-factor "${REPLICATION_FACTOR}" \
  --config "min.insync.replicas=${KAFKA_TOPIC_MIN_ISR}"
