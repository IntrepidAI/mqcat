## Installation

Download binary from [centrifugal/centrifugo](https://github.com/centrifugal/centrifugo/releases) (`centrifugo_6.3.1_linux_amd64.tar.gz` at the time of writing), unpack it and run `./centrifugo`.

Centrifugo refuses connections by default and requires a config file. Here's a simple config that allows everything:

```yaml
client:
  allow_anonymous_connect_without_token: true

channel:
  without_namespace:
    allow_subscribe_for_anonymous: true
    allow_subscribe_for_client: true
    allow_publish_for_anonymous: true
    allow_publish_for_client: true
```

```sh
$ ./centrifugo -c config.yaml
2025-09-29 08:50:35 [INF] using config file path=/home/user/centrifuge/config.yaml
2025-09-29 08:50:35 [INF] maxprocs: leaving gomaxprocs=12: cpu quota undefined
2025-09-29 08:50:35 [INF] starting Centrifugo engine=memory gomaxprocs=12 pid=1171 runtime=go1.24.7 version=6.3.1
2025-09-29 08:50:35 [INF] initializing engine engine_type=memory
2025-09-29 08:50:35 [INF] explicit broker not provided, using the one from engine
2025-09-29 08:50:35 [INF] explicit presence manager not provided, using the one from engine
2025-09-29 08:50:35 [INF] serving websocket, api endpoints on :8000
```

Run `mqcat` and make sure it connects (NOTE: `cfp` means `centrifuge protobuf`, use `cfj` for `centrifuge json` encoding).

```sh
$ mqcat cfp info
       Client ID: c25869d2-0b4e-4576-b7a6-4c9576587d92
  Server Version: 6.3.1 OSS
   Ping Interval: 25s
   Pong Required: true
   Token Expires: false
```

## Hello world

You can test publish/subscribe by running those commands in separate terminals.

```sh
$ mqcat cfp sub test_topic
$ mqcat cfp pub test_topic "Hello, World!"
```

You should see something like this:

```sh
$ mqcat cfp pub test_topic "Hello, World!"
2025-09-29T04:56:51.571066Z  INFO mqcat::cli: published 13 bytes to "test_topic"
```

```sh
$ mqcat cfp sub test_topic
[#1] Received on "test_topic" (13 bytes)
Hello, World!

```
