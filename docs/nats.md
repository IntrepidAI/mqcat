## Installation

Download binary from [nats-io/nats-server](https://github.com/nats-io/nats-server/releases) (`nats-server-v2.12.0-linux-amd64.tar.gz` at the time of writing), unpack it and run `./nats-server`.

```sh
$ ./nats-server
[184] 2025/09/29 04:44:15.412618 [INF] Starting nats-server
[184] 2025/09/29 04:44:15.412677 [INF]   Version:  2.12.0
[184] 2025/09/29 04:44:15.412681 [INF]   Git:      [fc6ec64]
[184] 2025/09/29 04:44:15.412703 [INF]   Name:     NCCHUKDGOB6VSE4LFDALSOOIAX2IGS7W5LDE667LZFHN567K4QKL7HE5
[184] 2025/09/29 04:44:15.412721 [INF]   ID:       NCCHUKDGOB6VSE4LFDALSOOIAX2IGS7W5LDE667LZFHN567K4QKL7HE5
[184] 2025/09/29 04:44:15.414486 [INF] Listening for client connections on 0.0.0.0:4222
[184] 2025/09/29 04:44:15.414822 [INF] Server is ready
```

Run `mqcat` and make sure it connects.

```sh
$ mqcat nats info
2025-09-29T04:27:28.902795Z  INFO async_nats::connector: connected successfully server=4222 max_payload=1048576
2025-09-29T04:27:28.903167Z  INFO async_nats: event: connected
          Client ID: 30
          Client IP: ::1
          Server ID: NCCHUKDGOB6VSE4LFDALSOOIAX2IGS7W5LDE667LZFHN567K4QKL7HE5
     Server Address: 0.0.0.0:4222
     Server Version: 2.12.0 (go1.25.1)
  Headers Supported: true
    Maximum Payload: 1048576
            Timeout: 10s
```

## Hello world

You can test publish/subscribe by running those commands in separate terminals.

```sh
$ mqcat nats sub test_topic
$ mqcat nats pub test_topic "Hello, World!"
```

You should see something like this:

```sh
$ mqcat nats pub test_topic "Hello, World!"
2025-09-29T00:51:19.541527Z  INFO async_nats::connector: connected successfully server=4222 max_payload=1048576
2025-09-29T00:51:19.541795Z  INFO async_nats: event: connected
2025-09-29T00:51:19.542665Z  INFO mqcat::cli: published 13 bytes to "test_topic"
```

```sh
$ mqcat nats sub test_topic
2025-09-29T00:50:36.480430Z  INFO async_nats::connector: connected successfully server=4222 max_payload=1048576
2025-09-29T00:50:36.480829Z  INFO async_nats: event: connected
[#1] Received on "test_topic" (13 bytes)
Hello, World!

```
