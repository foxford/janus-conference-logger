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
