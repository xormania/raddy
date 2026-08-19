use proptest::prelude::*;
use raddy_abi::{BodyMeta, Envelope, HeadCodec, JsonV1, RequestId, ResponseHead};
use std::net::SocketAddr;

fn header_pair() -> impl Strategy<Value = (String, String)> {
    (
        proptest::string::string_regex("[a-z][a-z0-9-]{0,15}").unwrap(),
        proptest::string::string_regex("[\x20-\x7e]{0,32}").unwrap(),
    )
}

fn envelope_strategy() -> impl Strategy<Value = Envelope> {
    (
        any::<u128>(),
        proptest::string::string_regex("[A-Z]{3,7}").unwrap(),
        proptest::string::string_regex("/[a-z0-9/_%?=&.-]{0,48}").unwrap(),
        prop_oneof!["http", "https"],
        proptest::string::string_regex("[a-z0-9.-]{1,24}:[0-9]{1,5}").unwrap(),
        proptest::collection::vec(header_pair(), 0..6),
        any::<u32>(),
        any::<u16>(),
        proptest::option::of(any::<u64>()),
        1u64..120_000,
    )
        .prop_map(
            |(entropy, method, target, scheme, authority, headers, ip, port, len, deadline_ms)| {
                let request_id = RequestId::from_u128(entropy);
                let remote_addr = SocketAddr::from(([127, 0, 0, 1], port.max(1)));
                let _ = ip;
                Envelope {
                    v: raddy_abi::ABI_VERSION,
                    request_id,
                    method,
                    target,
                    scheme: scheme.to_string(),
                    authority,
                    headers,
                    remote_addr,
                    body: BodyMeta { len },
                    deadline_ms,
                }
            },
        )
}

fn response_strategy() -> impl Strategy<Value = ResponseHead> {
    (100u16..600, proptest::collection::vec(header_pair(), 0..6))
        .prop_map(|(status, headers)| ResponseHead::new(status, headers))
}

proptest! {
    #[test]
    fn json_v1_round_trips_heads(env in envelope_strategy(), resp in response_strategy()) {
        let codec = JsonV1;
        let env_bytes = codec.encode_envelope(&env).expect("encode envelope");
        let env_back = codec.decode_envelope(&env_bytes).expect("decode envelope");
        prop_assert_eq!(env, env_back);

        let resp_bytes = codec.encode_response(&resp).expect("encode response");
        let resp_back = codec.decode_response(&resp_bytes).expect("decode response");
        prop_assert_eq!(resp, resp_back);
    }

    #[test]
    fn json_v1_hostile_input_never_panics(bytes in proptest::collection::vec(any::<u8>(), 0..256)) {
        let codec = JsonV1;
        let _ = codec.decode_envelope(&bytes);
        let _ = codec.decode_response(&bytes);
    }
}
