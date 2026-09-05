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

Sequences flatten using the item's index as a path segment, the same way a
map key would:

```
$ cat config.yaml
servers:
  - name: web1
    port: 80
  - name: web2
    port: 81
tags:
  - a
  - b

$ yaml-env-bridge --to env config.yaml
SERVERS__0__NAME=web1
SERVERS__0__PORT=80
SERVERS__1__NAME=web2
SERVERS__1__PORT=81
TAGS__0=a
TAGS__1=b
```

`--to yaml` reverses this: a group of children keyed exactly "0", "1", ...,
"n-1" is written back as a sequence rather than a mapping.

## Why streaming matters here

Config files are usually small, but this tool is also meant to work on the
generated ones - the giant flattened env dumps some platforms produce, or
YAML files assembled by templating. `--to env` reads its input one line at
a time with a `BufRead`, writes output incrementally, and only ever keeps
a small stack of the currently-open parent keys in memory (bounded by
nesting depth, not file size). A 5 KB config and a 5 GB one use the same
amount of memory.

`--to yaml` can't offer that same guarantee. Two `KEY=VALUE` lines that
belong under the same parent might not be adjacent - a hand-edited or
resorted env file, or one merged from several sources - and there's no way
to know a key group is complete until the input ends. So this direction
builds a tree of the keys seen so far (sized to the number of distinct
keys, not the byte size of the input) and writes the nested YAML once at
EOF. Non-contiguous groups are merged correctly; the cost is that this
direction is no longer bounded-memory streaming the way `--to env` is.

## Current limitations

This is an early skeleton, not a full YAML implementation:

- Only block-style mappings and block-style sequences of scalars or
  mappings are supported: no flow collections (`{a: 1}`, `[1, 2]`),
  anchors/aliases, nested lists-of-lists, or multi-line block scalars.
- Sequences and mappings that use small integers as keys are ambiguous by
  design: `--to yaml` treats any contiguous "0".."n-1" key group as a
  sequence, even if it started life as a mapping with those literal keys.

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
