FROM confluentinc/cp-kafka:7.5.5

USER root
RUN microdnf -y install jq curl \
  && microdnf clean all

COPY infra/kafka/init-kafka.sh /usr/local/bin/init-kafka.sh
COPY schemas/avro/MovieEvent.avsc /opt/schemas/avro/MovieEvent.avsc
RUN chmod +x /usr/local/bin/init-kafka.sh \
  && chmod -R a+rX /opt/schemas
ENV SCHEMA_FILE=/opt/schemas/avro/MovieEvent.avsc

USER appuser

ENTRYPOINT ["/usr/local/bin/init-kafka.sh"]
