use anyhow::Result;
pub use ignite_domain::decision::Model;

pub fn port(id: &str) -> Result<u16> {
    match id {
        "EU-A" | "EU-B" | "US-C" => ignite_settings::get().deployment_port(id),
        _ => anyhow::bail!("unknown endpoint"),
    }
}
