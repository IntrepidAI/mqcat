## Installation

Download binary from [eclipse-zenoh/zenoh](https://github.com/eclipse-zenoh/zenoh/releases) (`zenoh-1.5.1-x86_64-unknown-linux-gnu-standalone.zip` at the time of writing), unpack it and run `./zenohd`.

NOTE: this is optional, since Zenoh can do peer-to-peer communication without a router.

```sh
$ ./zenohd
2025-09-29T04:35:53.320105Z  INFO main ThreadId(01) zenohd: zenohd v1.5.1 built with rustc 1.85.0 (4d91de4e4 2025-02-17)
(...snip...)
2025-09-29T04:35:53.320546Z  INFO main ThreadId(01) zenoh::net::runtime: Using ZID: 220bdd01c3e5da98ea4dd7ddc902edaa
2025-09-29T04:35:53.321183Z  INFO main ThreadId(01) zenoh::net::runtime::orchestrator: Zenoh can be reached at: tcp/[fe80::215:5dff:febc:af16]:7447
2025-09-29T04:35:53.321207Z  INFO main ThreadId(01) zenoh::net::runtime::orchestrator: Zenoh can be reached at: tcp/172.24.63.58:7447
2025-09-29T04:35:53.321221Z  INFO main ThreadId(01) zenoh::net::runtime::orchestrator: zenohd listening scout messages on 224.0.0.224:7446
```

Run `mqcat` and make sure it connects.

```sh
$ mqcat zenoh info
2025-09-29T04:37:18.828513Z  INFO zenoh::net::runtime: Using ZID: c79d99c68f94791b845b44ab2d89a838
2025-09-29T04:37:18.829865Z  INFO zenoh::net::runtime::orchestrator: Zenoh can be reached at: tcp/[fe80::215:5dff:febc:af16]:42157
2025-09-29T04:37:18.829894Z  INFO zenoh::net::runtime::orchestrator: Zenoh can be reached at: tcp/172.24.63.58:42157
2025-09-29T04:37:18.829947Z  INFO zenoh::net::runtime::orchestrator: zenohd listening scout messages on 224.0.0.224:7446
            Client ID: c79d99c68f94791b845b44ab2d89a838
  Connected Router ID: 220bdd01c3e5da98ea4dd7ddc902edaa
2025-09-29T04:37:18.831253Z  INFO zenoh::api::session: close session zid=c79d99c68f94791b845b44ab2d89a838
```

## Hello world

You can test publish/subscribe by running those commands in separate terminals.

```sh
$ mqcat zenoh sub test_topic
$ mqcat zenoh pub test_topic "Hello, World!"
```

You should see something like this:

```sh
$ mqcat zenoh pub test_topic "Hello, World!"
2025-09-29T04:38:39.255674Z  INFO zenoh::net::runtime: Using ZID: ced8a17942969353a910f80d675d53c5
2025-09-29T04:38:39.256820Z  INFO zenoh::net::runtime::orchestrator: Zenoh can be reached at: tcp/[fe80::215:5dff:febc:af16]:33323
2025-09-29T04:38:39.256846Z  INFO zenoh::net::runtime::orchestrator: Zenoh can be reached at: tcp/172.24.63.58:33323
2025-09-29T04:38:39.256868Z  INFO zenoh::net::runtime::orchestrator: zenohd listening scout messages on 224.0.0.224:7446
2025-09-29T04:38:39.265444Z  INFO mqcat::cli: published 13 bytes to "test_topic"
2025-09-29T04:38:39.265472Z  INFO zenoh::api::session: close session zid=ced8a17942969353a910f80d675d53c5
```

```sh
$ mqcat zenoh sub test_topic
2025-09-29T04:38:32.961236Z  INFO zenoh::net::runtime: Using ZID: c2bbe80cb057cbd5d52ee1c8c9aeaff1
2025-09-29T04:38:32.962596Z  INFO zenoh::net::runtime::orchestrator: Zenoh can be reached at: tcp/[fe80::215:5dff:febc:af16]:35639
2025-09-29T04:38:32.962619Z  INFO zenoh::net::runtime::orchestrator: Zenoh can be reached at: tcp/172.24.63.58:35639
2025-09-29T04:38:32.962656Z  INFO zenoh::net::runtime::orchestrator: zenohd listening scout messages on 224.0.0.224:7446
[#1] Received on "test_topic" (13 bytes)
Hello, World!

```
