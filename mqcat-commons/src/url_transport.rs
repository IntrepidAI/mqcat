pub fn parse(url: &str) -> (&str, &str) {
    (|| -> Option<(&str, &str)> {
        let separator_pos = url.find(['+', '/', ':'])?;

        if url.get(separator_pos..=separator_pos)? == "+" {
            return Some((&url[..separator_pos], &url[separator_pos+1..]));
        }

        Some((&url[..separator_pos], url))
    })().unwrap_or((url, ""))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_url_transport_raw_transport() {
        assert_eq!(parse("zenoh"), ("zenoh", ""));
        assert_eq!(parse("nats"), ("nats", ""));
        assert_eq!(parse(""), ("", ""));
    }

    // #[test]
    // fn extract_url_transport_empty_url() {
    //     assert_eq!(parse("zenoh:"), ("zenoh", ""));
    //     assert_eq!(parse("nats://"), ("nats", ""));
    // }

    #[test]
    fn extract_url_transport_full_url() {
        assert_eq!(parse("nats://localhost:4222"), ("nats", "nats://localhost:4222"));
        assert_eq!(parse("ws://localhost:8080"), ("ws", "ws://localhost:8080"));
    }

    #[test]
    fn extract_url_transport_prefix() {
        assert_eq!(parse("zenoh+tcp/localhost:7447"), ("zenoh", "tcp/localhost:7447"));
        assert_eq!(parse("nats+ws://localhost:4222"), ("nats", "ws://localhost:4222"));
        assert_eq!(parse("cfj+ws://localhost:8000/connection/websocket"), ("cfj", "ws://localhost:8000/connection/websocket"));
    }

    #[test]
    fn extract_url_invalid() {
        assert_eq!(parse("+"), ("", ""));
        assert_eq!(parse("/"), ("", "/"));
        assert_eq!(parse(":"), ("", ":"));
    }
}
