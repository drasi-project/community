//! `ignite/assignment-plan` and `ignite/assignment-resilience`: native
//! transformers that assign items to bins with `good_lp` (microlp backend).
//! The plan factory emits `Assignment` elements and one `Plan`; the resilience
//! factory removes each healthy bin in turn and emits one `LossScenario` per
//! bin plus a `Resilience` summary.
//!
//! Contract: `plan/contracts.md` (Optimizer plugin). This crate is the phase-0
//! skeleton: factory metadata and configuration are final; component
//! construction is implemented by workstream 3.
//!
//! Layout: `config` and `solve` are plain Rust with host-free unit tests;
//! this file is the thin SDK wrapper (factories, plugin export).
use anyhow::Result;
use drasi_computation_contracts::*;
use drasi_computation_plugin_sdk::{
    Capabilities, ConfigSchema, ConfigType, ControlSender, CreateRequest, CreatedComponent,
    Factory, FactoryMetadata, NativeOutputSnapshots, PluginDefinition,
};
use std::sync::Arc;

pub mod config;
pub mod solve;
pub use config::{Config, Constraint, Mode, Objective, PLAN, RESILIENCE};

pub const ITEMS: &str = "items";
pub const BINS: &str = "bins";
pub const ELIGIBILITY: &str = "eligibility";
pub const OUT: &str = "out";

pub struct OptimizerFactory(pub Mode);
impl Factory for OptimizerFactory {
    fn describe(
        &self,
        request: &drasi_computation_plugin_sdk::wire::DescribeRequest,
    ) -> Result<drasi_computation_plugin_sdk::ComponentDescription> {
        Ok(request.volatile_transformer())
    }
    fn metadata(&self) -> FactoryMetadata {
        FactoryMetadata {
            implementation: ImplementationIdentity::try_new(self.0.implementation(), "1").unwrap(),
            role: ComponentRole::Transformer,
            configuration_version: 1,
            configuration: ConfigSchema::from_fields([
                ("items", ConfigType::Object, true),
                ("bins", ConfigType::Object, true),
                ("eligibility", ConfigType::Object, true),
                ("constraints", ConfigType::Array, true),
                ("objective", ConfigType::Array, true),
                ("epochProperty", ConfigType::String, false),
                ("coalesceMs", ConfigType::Integer, false),
                ("timeLimitMs", ConfigType::Integer, false),
            ]),
            ports: [ITEMS, BINS, ELIGIBILITY]
                .into_iter()
                .map(|name| (name, PortDirection::Input))
                .chain(std::iter::once((OUT, PortDirection::Output)))
                .map(|(name, direction)| {
                    PortDescriptor::new(
                        PortId::try_new(name).unwrap(),
                        direction,
                        GraphChangeCodec::schema().descriptor().clone(),
                        PipeRequirements::default(),
                    )
                })
                .collect(),
            completion: None,
            capabilities: Capabilities {
                continuations: true,
                startup_continuation: true,
                wakeups: true,
                input_binding_updates: true,
                ..Capabilities::default()
            },
        }
    }
    fn create(&self, _: &CreateRequest, _: ControlSender) -> Result<CreatedComponent> {
        anyhow::bail!("optimizer requires native upstream snapshots (an output_snapshots dependency)")
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
        anyhow::bail!("{}: not implemented yet (workstream 3)", self.0.implementation())
    }
}

pub fn plugin() -> Result<PluginDefinition> {
    PluginDefinition::new(
        "ignite-optimizer",
        env!("CARGO_PKG_VERSION"),
        vec![
            Arc::new(OptimizerFactory(Mode::Plan)),
            Arc::new(OptimizerFactory(Mode::Resilience)),
        ],
        vec![GraphChangeCodec::schema()],
    )
}
#[cfg(feature = "dynamic-plugin")]
drasi_computation_plugin_sdk::export_computation_plugin!(plugin());

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn example_configuration_is_valid_for_both_factories() {
        let example: serde_json::Value = serde_json::from_str(include_str!("../examples/ignite.json")).unwrap();
        for mode in [Mode::Plan, Mode::Resilience] {
            OptimizerFactory(mode).metadata().configuration.validate(&example).unwrap();
        }
        let config = Config::parse(&example).unwrap();
        assert_eq!(config.objective, [Objective::FewestMoves, Objective::LowestPeakUtilisation]);
        let mut shed = example.clone();
        shed["constraints"] = serde_json::json!(["binCapacity"]);
        assert!(Config::parse(&shed).is_err());
        assert!(plugin().is_ok());
    }
}
