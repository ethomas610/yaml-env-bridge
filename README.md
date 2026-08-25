# yaml-env-bridge

A small CLI for converting between nested YAML config files and flat
`KEY=VALUE` env files, in both directions.

The recurring problem: config for a service is naturally written as nested
YAML, but the place it has to end up is often flat - a container's env vars,
a systemd `EnvironmentFile`, a CI job's secrets. Hand-flattening a config
file every time it changes gets old, and doing it back the other way (turn
an `.env` dump into something readable) is worse.

## Usage

Flatten nested YAML into env-style assignments:

```
$ cat config.yaml
database:
  host: localhost
  port: 5432
  credentials:
    user: admin
debug: true

$ yaml-env-bridge --to env config.yaml
DATABASE__HOST=localhost
DATABASE__PORT=5432
DATABASE__CREDENTIALS__USER=admin
DEBUG=true
```

Expand it back into nested YAML:

```
$ yaml-env-bridge --to env config.yaml | yaml-env-bridge --to yaml
database:
  host: localhost
  port: 5432
  credentials:
    user: admin
debug: true
```

Both directions read from a file argument or stdin, and write to stdout or
a path given with `-o` / `--output`.

## Why streaming matters here

Config files are usually small, but this tool is also meant to work on the
generated ones - the giant flattened env dumps some platforms produce, or
YAML files assembled by templating. Both converters read their input one
line at a time with a `BufRead`, write output incrementally, and only ever
keep a small stack of the currently-open parent keys in memory (bounded by
nesting depth, not file size). A 5 KB config and a 5 GB one use the same
amount of memory.

## Current limitations

This is an early skeleton, not a full YAML implementation:

- Only block-style mappings of scalars are supported: no sequences, flow
  collections (`{a: 1}`), anchors/aliases, or multi-line block scalars.
- `--to yaml` assumes keys sharing a prefix arrive as consecutive lines,
  which is what `--to env` produces. Feeding it hand-written or reordered
  env files can produce a mapping key more than once in the output.
- Comment stripping is a straight-line heuristic, not a quote-aware
  scanner, so a `#` inside certain quoted strings can be misread as the
  start of a comment.

None of these are silent data loss - worst case is a rejected or malformed
YAML document you can inspect - but they're worth knowing about before
pointing this at something important.

## Building

Standard library only, no external crates:

```
cargo build --release
```

## License

MIT, see LICENSE.
