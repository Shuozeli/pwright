FROM rust:latest

RUN apt-get update && apt-get install -y protobuf-compiler curl dnsutils && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY . .

# Build integration tests
RUN cargo test -p pwright-integration-tests --no-run 2>&1 | tail -5

# Wait for Chrome then run tests.
# Chrome rejects Host headers that aren't localhost or an IP,
# so we resolve the hostname to IP for the health check.
CMD sh -c '\
  CHROME_IP=$(getent hosts "$CHROME_HOST" | awk "{print \$1}") && \
  echo "Chrome at $CHROME_HOST ($CHROME_IP):$CHROME_PORT" && \
  for i in $(seq 1 60); do \
    if curl -sf "http://$CHROME_IP:$CHROME_PORT/json/version" > /dev/null 2>&1; then \
      echo "Chrome ready!" && \
      failed=0; \
      for t in $(ls tests/integration/tests/*.rs | xargs -I{} basename {} .rs | grep -v lightpanda); do \
        cargo test -p pwright-integration-tests --test "$t" -- --ignored --test-threads=1 || failed=1; \
      done; \
      exit $failed; \
    fi; \
    sleep 1; \
    echo "  waiting... ($i/60)"; \
  done; \
  echo "Chrome not available" && exit 1'
