FROM confluentinc/cp-kafka:7.5.5

USER root

COPY infra/kafka/init-kafka.sh /usr/local/bin/init-kafka.sh
RUN chmod +x /usr/local/bin/init-kafka.sh

USER appuser

ENTRYPOINT ["/usr/local/bin/init-kafka.sh"]
