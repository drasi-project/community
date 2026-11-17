//! `ignite/prometheus-exporter`: a native sink that keeps the current rows of
//! each bound output and serves them in the Prometheus text format.
//!
//! Contract: `plan/contracts.md` §8 (Prometheus reaction) and §9 (metrics).
//!
//! Layout: `config`, `state` (current rows per output, snapshot and buffered
//! changes) and `text` (exposition format) are plain Rust with host-free unit
//! tests; `component` is the SDK lifecycle and HTTP listener; this file is the
//! factory.
use anyhow::Result;
use drasi_computation_contracts::*;
use drasi_computation_plugin_sdk::{
    Capabilities, Component, ConfigSchema, ConfigType, ControlSender, CreateRequest,
    CreatedComponent, Factory, FactoryMetadata, NativeOutputSnapshots, PluginDefinition,
};
use std::sync::Arc;

pub const IMPLEMENTATION: &str = "ignite/prometheus-exporter";
/// One input port; every bound output arrives on it (route by binding).
pub const IN: &str = "in";
mod component;
pub mod config;
pub mod state;
#[cfg(test)]
mod tests;
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
                // Volatile: there is no owned state, so an erasing dispose is a no-op.
                erases_owned_state: true,
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
        snapshots: Option<NativeOutputSnapshots>,
    ) -> Result<CreatedComponent> {
        self.metadata()
            .configuration
            .validate(&request.configuration)?;
        let config = Config::parse(&request.configuration)?;
        Ok(Component::Sink(Box::new(component::ExporterSink::new(
            self.metadata().descriptor(request.id.clone())?,
            config,
            &request.inputs,
            snapshots,
        )?))
        .into())
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
