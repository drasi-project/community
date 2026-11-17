//! `ignite/rego-policy`: a native transformer that evaluates a Rego rule for
//! every subject × target pair and emits one `PolicyDecision` per pair.
//!
//! Contract: `plan/contracts.md` (Rego policy plugin). This crate is the
//! phase-0 skeleton: factory metadata and configuration are final; component
//! construction is implemented by workstream 2.
//!
//! Layout: `config` and `evaluate` are plain Rust with host-free unit tests;
//! this file is the thin SDK wrapper (factory, plugin export).
use anyhow::Result;
use drasi_computation_contracts::*;
use drasi_computation_plugin_sdk::{
    Capabilities, ConfigSchema, ConfigType, ControlSender, CreateRequest, CreatedComponent,
    Factory, FactoryMetadata, NativeOutputSnapshots, PluginDefinition,
};
use std::sync::Arc;

pub mod config;
pub mod evaluate;
pub use config::Config;

pub const IMPLEMENTATION: &str = "ignite/rego-policy";
/// Input ports: one per role. Each may have any number of incoming edges.
pub const SUBJECTS: &str = "subjects";
pub const TARGETS: &str = "targets";
pub const PARAMETERS: &str = "parameters";
pub const OUT: &str = "out";
/// Label of every emitted element.
pub const LABEL: &str = "PolicyDecision";

pub struct PolicyFactory;
impl Factory for PolicyFactory {
    /// Stateless: outputs carry a new volatile producer identity on every construction.
    fn describe(
        &self,
        request: &drasi_computation_plugin_sdk::wire::DescribeRequest,
    ) -> Result<drasi_computation_plugin_sdk::ComponentDescription> {
        Ok(request.volatile_transformer())
    }
    fn metadata(&self) -> FactoryMetadata {
        FactoryMetadata {
            implementation: ImplementationIdentity::try_new(IMPLEMENTATION, "1").unwrap(),
            role: ComponentRole::Transformer,
            configuration_version: 1,
            configuration: ConfigSchema::from_fields([
                ("module", ConfigType::String, true),
                ("rule", ConfigType::String, true),
                ("subjects", ConfigType::Object, true),
                ("targets", ConfigType::Object, true),
                ("parameters", ConfigType::Object, true),
                ("epochProperty", ConfigType::String, false),
                ("timeoutMs", ConfigType::Integer, false),
            ]),
            ports: ports(&[SUBJECTS, TARGETS, PARAMETERS], OUT),
            completion: None,
            capabilities: Capabilities {
                continuations: true,
                startup_continuation: true,
                input_binding_updates: true,
                ..Capabilities::default()
            },
        }
    }
    fn create(&self, _: &CreateRequest, _: ControlSender) -> Result<CreatedComponent> {
        anyhow::bail!("rego-policy requires native upstream snapshots (an output_snapshots dependency)")
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
        anyhow::bail!("{IMPLEMENTATION}: not implemented yet (workstream 2)")
    }
}

pub(crate) fn ports(inputs: &[&str], output: &str) -> Vec<PortDescriptor> {
    inputs
        .iter()
        .map(|name| (*name, PortDirection::Input))
        .chain(std::iter::once((output, PortDirection::Output)))
        .map(|(name, direction)| {
            PortDescriptor::new(
                PortId::try_new(name).unwrap(),
                direction,
                GraphChangeCodec::schema().descriptor().clone(),
                PipeRequirements::default(),
            )
        })
        .collect()
}

pub fn plugin() -> Result<PluginDefinition> {
    PluginDefinition::new(
        "ignite-policy",
        env!("CARGO_PKG_VERSION"),
        vec![Arc::new(PolicyFactory)],
        vec![GraphChangeCodec::schema()],
    )
}
#[cfg(feature = "dynamic-plugin")]
drasi_computation_plugin_sdk::export_computation_plugin!(plugin());

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, Value};

    const MODULE: &str = include_str!("../ignite.rego");

    fn example() -> Value {
        let mut value: Value = serde_json::from_str(include_str!("../examples/ignite.json")).unwrap();
        value["module"] = json!(MODULE);
        value
    }

    #[test]
    fn example_configuration_is_valid() {
        let factory = PolicyFactory;
        factory.metadata().configuration.validate(&example()).unwrap();
        let config = Config::parse(&example()).unwrap();
        assert_eq!(config.parameters.target_key, "deployment");
        let mut wrong = example();
        wrong["unknown"] = json!(1);
        assert!(factory.metadata().configuration.validate(&wrong).is_err() || Config::parse(&wrong).is_err());
        assert!(plugin().is_ok());
        let ports: Vec<_> = factory.metadata().ports.iter().map(|p| p.id().as_str().to_owned()).collect();
        assert_eq!(ports, [SUBJECTS, TARGETS, PARAMETERS, OUT]);
    }
}
