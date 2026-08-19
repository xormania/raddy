use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::path::PathBuf;

use figment::providers::{Format, Serialized, Toml};
use figment::value::{Dict, Map, Value};
use figment::{Error as FigmentError, Figment, Metadata, Profile, Provider};

use crate::ConfigError;
use crate::schema::Config;

/// Where the TOML file layer comes from.
#[derive(Debug, Clone, Default)]
pub enum FileSource {
    /// No file layer.
    #[default]
    None,
    /// Required path; missing file is an error.
    Required(PathBuf),
    /// Used when present, skipped when absent.
    Optional(PathBuf),
    /// In-memory TOML (tests and callers that already read the file).
    Toml(String),
}

/// Where the `RADDY_*` overlay comes from.
#[derive(Debug, Clone, Default)]
pub enum EnvSource {
    #[default]
    None,
    /// Process environment, filtered to `RADDY_*`.
    Process,
    /// Explicit map, same key convention as the process (`RADDY_SERVER_LISTEN`).
    Map(BTreeMap<String, String>),
}

/// CLI overlay. Only `Some` fields participate, so absent flags do not reset defaults.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CliOverrides {
    pub server_listen: Option<SocketAddr>,
    pub admin_listen: Option<SocketAddr>,
    pub artifact: Option<PathBuf>,
    pub concurrency: Option<u32>,
}

/// One load: compiled defaults, then file, then env, then CLI. Rightmost wins.
#[derive(Debug, Clone, Default)]
pub struct LoadRequest {
    pub file: FileSource,
    pub env: EnvSource,
    pub cli: CliOverrides,
    /// Lookup table for `env:NAME` string values.
    pub secrets: BTreeMap<String, String>,
}

/// Load and validate a [`Config`].
///
/// Layers, left to right, rightmost wins: compiled defaults → file → `RADDY_*` → CLI.
pub fn load(req: LoadRequest) -> Result<Config, ConfigError> {
    let mut figment = Figment::from(Serialized::defaults(Config::default()));

    if let Some(text) = read_file_source(&req.file)? {
        // Figment extract does not honor `deny_unknown_fields`; serde on the raw TOML does.
        reject_unknown_toml(&text)?;
        figment = figment.merge(Toml::string(&text));
    }

    let env = collect_env(&req.env);
    let env_dict = env_to_dict(&env)?;
    figment = figment.merge(DictProvider {
        name: "environment",
        dict: env_dict,
    });
    figment = figment.merge(DictProvider {
        name: "cli",
        dict: cli_to_dict(&req.cli),
    });

    let mut cfg: Config = figment.extract()?;
    resolve_secrets(&mut cfg, &req.secrets)?;
    validate(&cfg)?;
    Ok(cfg)
}

fn read_file_source(src: &FileSource) -> Result<Option<String>, ConfigError> {
    match src {
        FileSource::None => Ok(None),
        FileSource::Toml(text) => Ok(Some(text.clone())),
        FileSource::Required(path) => {
            std::fs::read_to_string(path)
                .map(Some)
                .map_err(|source| ConfigError::Io {
                    path: path.clone(),
                    source,
                })
        }
        FileSource::Optional(path) => match std::fs::read_to_string(path) {
            Ok(text) => Ok(Some(text)),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(ConfigError::Io {
                path: path.clone(),
                source,
            }),
        },
    }
}

fn reject_unknown_toml(text: &str) -> Result<(), ConfigError> {
    match toml::from_str::<Config>(text) {
        Ok(_) => Ok(()),
        Err(err) => {
            let msg = err.to_string();
            if msg.contains("unknown field") {
                Err(ConfigError::UnknownKey(msg))
            } else {
                Err(ConfigError::Parse(err))
            }
        }
    }
}

fn collect_env(src: &EnvSource) -> BTreeMap<String, String> {
    match src {
        EnvSource::None => BTreeMap::new(),
        EnvSource::Process => std::env::vars().collect(),
        EnvSource::Map(map) => map.clone(),
    }
}

fn env_to_dict(env: &BTreeMap<String, String>) -> Result<Dict, ConfigError> {
    let mut dict = Dict::new();
    for (key, raw) in env {
        if let Some(path) = parse_raddy_key(key)? {
            let path: Vec<&str> = path.iter().map(String::as_str).collect();
            insert_path(&mut dict, &path, coerce_env_value(raw));
        }
    }
    Ok(dict)
}

fn cli_to_dict(cli: &CliOverrides) -> Dict {
    let mut dict = Dict::new();
    if let Some(addr) = cli.server_listen {
        insert_path(
            &mut dict,
            &["server", "listen"],
            Value::from(addr.to_string()),
        );
    }
    if let Some(addr) = cli.admin_listen {
        insert_path(
            &mut dict,
            &["admin", "listen"],
            Value::from(addr.to_string()),
        );
    }
    if let Some(path) = &cli.artifact {
        insert_path(
            &mut dict,
            &["executor", "artifact"],
            Value::from(path.to_string_lossy().into_owned()),
        );
    }
    if let Some(n) = cli.concurrency {
        insert_path(&mut dict, &["server", "concurrency"], Value::from(n));
    }
    dict
}

struct Section {
    prefix: &'static str,
    name: &'static str,
    fields: &'static [&'static str],
}

const SECTIONS: &[Section] = &[
    Section {
        prefix: "server_",
        name: "server",
        fields: &["listen", "concurrency", "request_timeout_ms"],
    },
    Section {
        prefix: "admin_",
        name: "admin",
        fields: &["listen"],
    },
    Section {
        prefix: "executor_",
        name: "executor",
        fields: &[
            "artifact",
            "pool_min",
            "pool_max",
            "deadline_ms",
            "epoch_tick_ms",
            "teardown_deadline_ms",
            "debug_fuel",
        ],
    },
    Section {
        prefix: "observability_",
        name: "observability",
        fields: &["log_format", "trace_level"],
    },
];

/// Map `RADDY_SERVER_LISTEN` → `["server", "listen"]`.
///
/// Split-on-every-`_` is wrong for fields that themselves contain underscores
/// (`request_timeout_ms`). Section prefixes are matched first; the remainder is
/// the field name. Any other `RADDY_*` key is unknown (C5). The BDD slice
/// selector is `BDD_STAGE`, outside this prefix.
fn parse_raddy_key(key: &str) -> Result<Option<Vec<String>>, ConfigError> {
    let Some(rest) = key.strip_prefix("RADDY_") else {
        return Ok(None);
    };
    let rest = rest.to_ascii_lowercase();

    for section in SECTIONS {
        if let Some(field) = rest.strip_prefix(section.prefix) {
            if section.fields.contains(&field) {
                return Ok(Some(vec![section.name.to_string(), field.to_string()]));
            }
            return Err(ConfigError::UnknownKey(key.to_string()));
        }
    }

    if let Some(after) = rest.strip_prefix("db_") {
        for field in ["pool_max", "driver", "url"] {
            let suffix = format!("_{field}");
            if let Some(name) = after.strip_suffix(suffix.as_str())
                && !name.is_empty()
            {
                return Ok(Some(vec!["db".into(), name.into(), field.into()]));
            }
        }
        return Err(ConfigError::UnknownKey(key.to_string()));
    }

    Err(ConfigError::UnknownKey(key.to_string()))
}

fn coerce_env_value(raw: &str) -> Value {
    if raw.eq_ignore_ascii_case("true") {
        return Value::from(true);
    }
    if raw.eq_ignore_ascii_case("false") {
        return Value::from(false);
    }
    if let Ok(n) = raw.parse::<i64>() {
        return Value::from(n);
    }
    Value::from(raw)
}

fn insert_path(dict: &mut Dict, path: &[&str], value: Value) {
    match path {
        [] => {}
        [leaf] => {
            dict.insert((*leaf).to_string(), value);
        }
        [head, rest @ ..] => {
            let entry = dict
                .entry((*head).to_string())
                .or_insert_with(|| Value::from(Dict::new()));
            match entry {
                Value::Dict(_, child) => insert_path(child, rest, value),
                _ => {
                    let mut child = Dict::new();
                    insert_path(&mut child, rest, value);
                    *entry = Value::from(child);
                }
            }
        }
    }
}

struct DictProvider {
    name: &'static str,
    dict: Dict,
}

impl Provider for DictProvider {
    fn metadata(&self) -> Metadata {
        Metadata::named(self.name)
    }

    fn data(&self) -> Result<Map<Profile, Dict>, FigmentError> {
        Ok(Map::from([(Profile::Default, self.dict.clone())]))
    }
}

fn resolve_secrets(
    cfg: &mut Config,
    secrets: &BTreeMap<String, String>,
) -> Result<(), ConfigError> {
    let artifact = cfg.executor.artifact.to_string_lossy();
    if artifact.starts_with("env:") {
        cfg.executor.artifact = resolve_env_ref(artifact.as_ref(), secrets)?.into();
    }
    for db in cfg.db.values_mut() {
        db.url = resolve_env_ref(&db.url, secrets)?;
    }
    Ok(())
}

fn resolve_env_ref(raw: &str, secrets: &BTreeMap<String, String>) -> Result<String, ConfigError> {
    let Some(name) = raw.strip_prefix("env:") else {
        return Ok(raw.to_string());
    };
    if name.is_empty() || name.chars().any(char::is_whitespace) {
        return Err(ConfigError::BadEnvRef(raw.to_string()));
    }
    secrets
        .get(name)
        .cloned()
        .ok_or_else(|| ConfigError::MissingEnv(name.to_string()))
}

fn validate(cfg: &Config) -> Result<(), ConfigError> {
    if cfg.server.concurrency == 0 {
        return Err(ConfigError::Concurrency);
    }
    if cfg.executor.pool_min > cfg.executor.pool_max {
        return Err(ConfigError::PoolBounds {
            min: cfg.executor.pool_min,
            max: cfg.executor.pool_max,
        });
    }
    Ok(())
}
