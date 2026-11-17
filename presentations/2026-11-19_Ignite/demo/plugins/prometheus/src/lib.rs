//! `ignite/prometheus-exporter`: a native sink that keeps the current rows of
//! each bound output and serves them in the Prometheus text format.
//!
//! Contract: `plan/contracts.md` (Prometheus reaction). This crate is the
//! phase-0 skeleton: factory metadata and configuration are final; component
//! construction is implemented by workstream 4.
//!
//! Layout: `config` and `text` (exposition format) are plain Rust with
//! host-free unit tests; this file is the thin SDK wrapper.
use anyhow::Result;
use drasi_computation_contracts::*;
use drasi_computation_plugin_sdk::{
    Capabilities, ConfigSchema, ConfigType, ControlSender, CreateRequest, CreatedComponent,
    Factory, FactoryMetadata, NativeOutputSnapshots, PluginDefinition,
};
use std::sync::Arc;

pub const IMPLEMENTATION: &str = "ignite/prometheus-exporter";
/// One input port; every bound output arrives on it (route by binding).
pub const IN: &str = "in";
pub mod config;
pub mod text;
pub use config::{Config, READY_METRIC};

pub struct ExporterFactory;
impl Factory for ExporterFactory {
    fn describe(
        &self,
        request: &drasi_computation_plugin_sdk::wire::DescribeRequest,
    ) -> Result<drasi_computation_plugin_sdk::ComponentDescription> {
        Ok(request.unkeyed_sink())
    }
    fn metadata(&self) -> FactoryMetadata {
        FactoryMetadata {
            implementation: ImplementationIdentity::try_new(IMPLEMENTATION, "1").unwrap(),
            role: ComponentRole::Sink,
            configuration_version: 1,
            configuration: ConfigSchema::from_fields([
                ("listen", ConfigType::String, true),
                ("path", ConfigType::String, false),
                ("outputs", ConfigType::Object, true),
                ("metrics", ConfigType::Object, true),
            ]),
            ports: vec![PortDescriptor::new(
                PortId::try_new(IN).unwrap(),
                PortDirection::Input,
                GraphChangeCodec::schema().descriptor().clone(),
                PipeRequirements::default(),
            )],
            completion: Some(SinkCompletion::Accepted),
            capabilities: Capabilities {
                binds_inputs: true,
                ..Capabilities::default()
            },
        }
    }
    fn create(&self, request: &CreateRequest, control: ControlSender) -> Result<CreatedComponent> {
        self.create_with_output_snapshots(request, control, None)
    }
    fn supports_output_snapshots(&self) -> bool {
        true
    }
    fn create_with_output_snapshots(
        &self,
        request: &CreateRequest,
        _: ControlSender,
        _snapshots: Option<NativeOutputSnapshots>,
    ) -> Result<CreatedComponent> {
        self.metadata().configuration.validate(&request.configuration)?;
        Config::parse(&request.configuration)?;
        anyhow::bail!("{IMPLEMENTATION}: not implemented yet (workstream 4)")
    }
}

pub fn plugin() -> Result<PluginDefinition> {
    PluginDefinition::new(
        "ignite-prometheus",
        env!("CARGO_PKG_VERSION"),
        vec![Arc::new(ExporterFactory)],
        vec![GraphChangeCodec::schema()],
    )
}
#[cfg(feature = "dynamic-plugin")]
drasi_computation_plugin_sdk::export_computation_plugin!(plugin());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_configuration_is_valid() {
        let example: serde_json::Value = serde_json::from_str(include_str!("../examples/ignite.json")).unwrap();
        ExporterFactory.metadata().configuration.validate(&example).unwrap();
        let config = Config::parse(&example).unwrap();
        assert_eq!(config.outputs.len(), config.metrics.len());
        let mut bad = example.clone();
        bad["metrics"]["overdue"][0]["metric"] = serde_json::json!("ignite-drasi");
        assert!(Config::parse(&bad).is_err());
        assert!(plugin().is_ok());
    }
}
