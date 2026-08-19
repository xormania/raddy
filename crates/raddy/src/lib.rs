//! In-process entry used by the binary and the BDD harness.

use std::collections::BTreeMap;
use std::path::PathBuf;

use raddy_config::{CliOverrides, EnvSource, FileSource, LoadRequest};

/// Result of one in-process invocation of the `raddy` CLI surface.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invocation {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// Run the CLI against an explicit environment map (no process-env side effects).
pub fn invoke(args: &[String], env: &BTreeMap<String, String>) -> Invocation {
    match invoke_inner(args, env) {
        Ok(out) => out,
        Err(err) => Invocation {
            stdout: String::new(),
            stderr: format!("{err}\n"),
            exit_code: 2,
        },
    }
}

fn invoke_inner(args: &[String], env: &BTreeMap<String, String>) -> Result<Invocation, String> {
    let parsed = parse_args(args)?;
    if parsed.help {
        return Ok(Invocation {
            stdout: help_text().into(),
            stderr: String::new(),
            exit_code: 0,
        });
    }

    let file = match parsed.config {
        Some(path) => FileSource::Required(path),
        None => FileSource::Optional(PathBuf::from("raddy.toml")),
    };
    let cfg = raddy_config::load(LoadRequest {
        file,
        env: EnvSource::Map(env.clone()),
        cli: parsed.overrides,
        secrets: env.clone(),
    })
    .map_err(|err| err.to_string())?;

    if parsed.print_config {
        return Ok(Invocation {
            stdout: cfg.to_toml().map_err(|err| err.to_string())?,
            stderr: String::new(),
            exit_code: 0,
        });
    }

    Ok(Invocation {
        stdout: String::new(),
        stderr: "raddy: HTTP server is not implemented yet\n".into(),
        exit_code: 2,
    })
}

#[derive(Debug, Default)]
struct ParsedArgs {
    help: bool,
    print_config: bool,
    config: Option<PathBuf>,
    overrides: CliOverrides,
}

fn parse_args(args: &[String]) -> Result<ParsedArgs, String> {
    let mut parsed = ParsedArgs::default();
    let mut iter = args.iter();
    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => parsed.help = true,
            "--print-config" => parsed.print_config = true,
            "--config" => {
                let value = iter.next().ok_or("--config requires a path")?;
                parsed.config = Some(PathBuf::from(value));
            }
            "--listen" => {
                let value = iter.next().ok_or("--listen requires an address")?;
                parsed.overrides.server_listen =
                    Some(value.parse().map_err(|err| format!("--listen: {err}"))?);
            }
            "--admin-listen" => {
                let value = iter.next().ok_or("--admin-listen requires an address")?;
                parsed.overrides.admin_listen = Some(
                    value
                        .parse()
                        .map_err(|err| format!("--admin-listen: {err}"))?,
                );
            }
            "--artifact" => {
                let value = iter.next().ok_or("--artifact requires a path")?;
                parsed.overrides.artifact = Some(PathBuf::from(value));
            }
            "--concurrency" => {
                let value = iter.next().ok_or("--concurrency requires an integer")?;
                parsed.overrides.concurrency = Some(
                    value
                        .parse()
                        .map_err(|err| format!("--concurrency: {err}"))?,
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    Ok(parsed)
}

fn help_text() -> &'static str {
    "\
raddy [options]
  --print-config         print the fully-resolved config as TOML
  --config PATH          config file (default: raddy.toml if present)
  --listen ADDR          override server.listen
  --admin-listen ADDR    override admin.listen
  --artifact PATH        override executor.artifact
  --concurrency N        override server.concurrency
"
}

/// Parse `std::env::args()` after the binary name and invoke.
pub fn run_from_process() -> Invocation {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let env: BTreeMap<String, String> = std::env::vars().collect();
    invoke(&args, &env)
}
