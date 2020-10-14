# janus-conference-logger

A custom contextual Janus Gateway logger plugin.

## Build & install

```bash
cargo build --release
mkdir -p /opt/janus/lib/janus/loggers
cp target/release/libjanus_conference_logger.so /opt/janus/lib/loggers/
```

In `/opt/janus/etc/janus/janus.cfg` disable default logging to avoid mess up:

```jcfg
general: {
  debug_colors = false
  debug_timestamps = false
  log_to_stdout = false
}
```

## Docker

```bash
VERSION=`cargo read-manifest | jq -r '.version'`
docker build -f docker/Dockerfile -t netologygroup/janus-conference-logger:v${VERSION} .
```

Then copy the build artifact into the target Janus Gateway image:

```dockerfile
COPY --from=netologygroup/janus-conference-logger:v${VERSION} \
    /build/target/release/libjanus_conference_logger.so \
    /opt/janus/lib/janus/loggers/libjanus_conference_logger.so
```
