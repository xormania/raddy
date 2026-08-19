use std::collections::BTreeMap;
use std::net::SocketAddr;

use proptest::prelude::*;
use raddy_config::{CliOverrides, EnvSource, FileSource, LoadRequest, load};

const ADDRS: &[&str] = &[
    "127.0.0.1:1",
    "127.0.0.1:8080",
    "0.0.0.0:9090",
    "10.1.2.3:65535",
];

fn addr_idx() -> impl Strategy<Value = usize> {
    0..ADDRS.len()
}

fn opt_addr() -> impl Strategy<Value = Option<usize>> {
    proptest::option::of(addr_idx())
}

fn opt_concurrency() -> impl Strategy<Value = Option<u32>> {
    proptest::option::of(1u32..=512)
}

proptest! {
    /// Any combination of file / env / CLI: the rightmost present source wins.
    #[test]
    fn layering_is_rightmost_wins(
        file in opt_addr(),
        env in opt_addr(),
        cli in opt_addr(),
        file_c in opt_concurrency(),
        env_c in opt_concurrency(),
        cli_c in opt_concurrency(),
    ) {
        let default_listen: SocketAddr = "127.0.0.1:8080".parse().unwrap();
        let expected_listen = cli
            .or(env)
            .or(file)
            .map(|i| ADDRS[i].parse::<SocketAddr>().unwrap())
            .unwrap_or(default_listen);
        let expected_concurrency = cli_c.or(env_c).or(file_c).unwrap_or(256);

        let file_src = if file.is_some() || file_c.is_some() {
            let mut body = String::from("[server]\n");
            if let Some(i) = file {
                body.push_str(&format!("listen = \"{}\"\n", ADDRS[i]));
            }
            if let Some(c) = file_c {
                body.push_str(&format!("concurrency = {c}\n"));
            }
            FileSource::Toml(body)
        } else {
            FileSource::None
        };

        let env_src = {
            let mut map = BTreeMap::new();
            if let Some(i) = env {
                map.insert("RADDY_SERVER_LISTEN".into(), ADDRS[i].into());
            }
            if let Some(c) = env_c {
                map.insert("RADDY_SERVER_CONCURRENCY".into(), c.to_string());
            }
            if map.is_empty() {
                EnvSource::None
            } else {
                EnvSource::Map(map)
            }
        };

        let cfg = load(LoadRequest {
            file: file_src,
            env: env_src,
            cli: CliOverrides {
                server_listen: cli.map(|i| ADDRS[i].parse().unwrap()),
                concurrency: cli_c,
                ..CliOverrides::default()
            },
            secrets: BTreeMap::new(),
        })
        .expect("layered inputs in this test are always valid");

        prop_assert_eq!(cfg.server.listen, expected_listen);
        prop_assert_eq!(cfg.server.concurrency, expected_concurrency);
    }
}
