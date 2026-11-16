use anyhow::Result;
pub use ignite_domain::decision::Model;

pub fn port(id: &str) -> Result<u16> {
    match id {
        "EU-A" => Ok(18531),
        "EU-B" => Ok(18532),
        "US-C" => Ok(18533),
        _ => anyhow::bail!("unknown endpoint"),
    }
}
