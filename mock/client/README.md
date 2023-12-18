# Logdna Mock Client
This executable produces a lot of log lines.
Build it here with `docker build -t mock_client -f Dockerfile ..`.
Configure it in the `docker-compose.yaml` and run with Docker Compose.
Make sure that the logdna plugin is installed, the [logdna mock server](../server) started and enjoy the wall of text.

