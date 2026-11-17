use anyhow::Result;

/// Listening port of a simulated model deployment.
pub fn port(id: &str) -> Result<u16> {
    match id {
        "EU-A" | "EU-B" | "US-C" => ignite_settings::get().deployment_port(id),
        _ => anyhow::bail!("unknown deployment"),
    }
}
