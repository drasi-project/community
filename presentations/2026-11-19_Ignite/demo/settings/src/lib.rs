//! The demo's ports, names and keys, from `demo/ports.env` (one place for all of them).
//!
//! `IGNITE_SETTINGS=<file>` selects another file; an exported variable with the
//! same name as a line in the file overrides that line.
use anyhow::{bail, Context, Result};
use std::{collections::BTreeMap, path::PathBuf, sync::OnceLock};

#[derive(Clone, Debug)]
pub struct Settings(BTreeMap<String, String>);

impl Settings {
    pub fn load() -> Result<Self> {
        let demo = PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/.."));
        let path = std::env::var_os("IGNITE_SETTINGS")
            .map(PathBuf::from)
            .map(|path| if path.is_absolute() { path } else { demo.join(path) })
            .unwrap_or_else(|| demo.join("ports.env"));
        let text = std::fs::read_to_string(&path)
            .with_context(|| format!("read demo settings {}", path.display()))?;
        Self::parse(&text, |key| std::env::var(key).ok().filter(|v| !v.is_empty()))
    }

    pub fn parse(text: &str, environment: impl Fn(&str) -> Option<String>) -> Result<Self> {
        let mut values = BTreeMap::new();
        for line in text.lines().map(str::trim) {
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                bail!("demo settings: expected KEY=VALUE, got {line:?}");
            };
            let key = key.trim().to_owned();
            let value = environment(&key).unwrap_or_else(|| value.trim().to_owned());
            values.insert(key, value);
        }
        Ok(Self(values))
    }

    pub fn get(&self, key: &str) -> Result<&str> {
        self.0
            .get(key)
            .map(String::as_str)
            .with_context(|| format!("demo settings have no {key}"))
    }

    /// `port("WORKLOAD")` reads `IGNITE_WORKLOAD_PORT`.
    pub fn port(&self, name: &str) -> Result<u16> {
        let key = format!("IGNITE_{name}_PORT");
        self.get(&key)?
            .parse()
            .with_context(|| format!("{key} must be a TCP port"))
    }

    /// `http://127.0.0.1:<port>` for a local service.
    pub fn url(&self, name: &str) -> Result<String> {
        Ok(format!("http://127.0.0.1:{}", self.port(name)?))
    }

    /// The port of a simulated deployment: `EU-A` reads `IGNITE_DEPLOYMENT_EU_A_PORT`.
    pub fn deployment_port(&self, deployment: &str) -> Result<u16> {
        self.port(&format!("DEPLOYMENT_{}", deployment.replace('-', "_")))
    }
}

/// The settings for this process, loaded once. Panics with the reason if the
/// file is missing or malformed: nothing in the demo can run without it.
pub fn get() -> &'static Settings {
    static SETTINGS: OnceLock<Settings> = OnceLock::new();
    SETTINGS.get_or_init(|| Settings::load().unwrap_or_else(|error| panic!("{error:#}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_file_with_environment_overrides() {
        let text = "# comment\nIGNITE_WORKLOAD_PORT=19500\nIGNITE_DEPLOYMENT_EU_A_PORT=19531\nIGNITE_PROJECT=x\n";
        let settings = Settings::parse(text, |key| {
            (key == "IGNITE_PROJECT").then(|| "override".to_owned())
        })
        .unwrap();
        assert_eq!(settings.port("WORKLOAD").unwrap(), 19500);
        assert_eq!(settings.deployment_port("EU-A").unwrap(), 19531);
        assert_eq!(settings.get("IGNITE_PROJECT").unwrap(), "override");
        assert!(settings.port("MISSING").is_err());
        assert!(Settings::parse("not a setting", |_| None).is_err());
    }

    #[test]
    fn both_committed_files_define_the_same_settings() {
        let read = |name: &str| {
            let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../").to_owned() + name).unwrap();
            Settings::parse(&text, |_| None).unwrap().0
        };
        let (copy, original) = (read("ports.env"), read("ports.default.env"));
        assert_eq!(copy.keys().collect::<Vec<_>>(), original.keys().collect::<Vec<_>>());
        // This copy shares no port, name or key with the original demo.
        let taken: std::collections::BTreeSet<_> = original.values().collect();
        for (key, value) in &copy {
            assert!(!taken.contains(value), "{key}={value} is used by the original demo");
        }
    }
}
