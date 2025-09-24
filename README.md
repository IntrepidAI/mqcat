# mqcat

Command-line client for pub/sub messaging systems like zenoh, nats, mqtt, and centrifuge.

## Why?

NATS has a great CLI tool, Zenoh is lacking one at the moment (python one is bit too slow), and Centrifugo doesn't have one at all.

I wanted to have the same experience as NATS CLI with other protocols/brokers, so I wrote this tool.

## Installation

Option 1 - download new binary:
 - download the binary from the [releases](https://github.com/IntrepidAI/mqcat/releases) page

Option 2 - update existing binary:
 - run `mqcat --upgrade` to upgrade to the latest version

Option 3 - compile from source code:
 - run `cargo install mqcat` to install the latest version from source code (if you have Rust installed)

## Usage

### zenoh

```sh
# publish a message to zenoh
mqcat zenoh pub 'test' 'Hello, world!'

# subscribe to zenoh topics
mqcat zenoh sub '**'

# connect to specific zenoh server
mqcat zenoh+tcp/localhost:7447 sub 'test'
```

### nats

```sh
# publish a message to nats
mqcat nats pub 'test' 'Hello, world!'

# subscribe to nats topics
mqcat nats sub '>'

# connect to specific nats server
mqcat nats://localhost:4222 sub 'test'
```
