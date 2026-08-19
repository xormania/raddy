use cucumber::{given, then, when};
use raddy_config::Config;

use crate::BddWorld;

#[given(regex = r#"^the environment variable "([^"]+)" is "([^"]+)"$"#)]
async fn set_env(world: &mut BddWorld, key: String, value: String) {
    world.env.insert(key, value);
}

#[when(regex = r#"^I run raddy with "([^"]+)"$"#)]
async fn run_raddy(world: &mut BddWorld, args: String) {
    let argv: Vec<String> = args.split_whitespace().map(str::to_string).collect();
    let out = raddy::invoke(&argv, &world.env);
    assert_eq!(
        out.exit_code, 0,
        "raddy exited {}: {}",
        out.exit_code, out.stderr
    );
    world.printed = Some(out.stdout);
}

#[then(expr = "the printed config parses as TOML")]
async fn printed_parses(world: &mut BddWorld) {
    let printed = world
        .printed
        .as_deref()
        .expect("a When step must print config before this Then");
    let parsed: Config = toml::from_str(printed).expect("printed config must be valid TOML");
    world.parsed = Some(parsed);
}

#[then(regex = r#"^the printed server.listen is "([^"]+)"$"#)]
async fn printed_listen(world: &mut BddWorld, expected: String) {
    let cfg = world
        .parsed
        .as_ref()
        .expect("the printed config must parse before this Then");
    assert_eq!(cfg.server.listen.to_string(), expected);
}

#[then(expr = "the printed config round-trips through the TOML parser")]
async fn printed_round_trips(world: &mut BddWorld) {
    let first = world
        .parsed
        .as_ref()
        .expect("the printed config must parse before this Then");
    let again = first
        .to_toml()
        .expect("resolved config must serialize to TOML");
    let second: Config = toml::from_str(&again).expect("re-emitted TOML must parse");
    assert_eq!(first, &second);
}
