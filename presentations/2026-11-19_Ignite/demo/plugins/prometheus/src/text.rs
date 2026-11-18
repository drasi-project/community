//! Prometheus text exposition format (version 0.0.4): plain Rust, no host
//! types. The component (lib.rs) turns its current rows into [`Family`]s and
//! serves `render` on the configured path.
use std::collections::BTreeMap;

/// One metric name with its HELP text and current series (all gauges).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Family {
    pub name: String,
    pub help: String,
    /// label set -> value; ordered so output is deterministic.
    pub series: BTreeMap<Vec<(String, String)>, f64>,
}

pub fn render(families: &[Family]) -> String {
    let mut out = String::new();
    for family in families {
        out.push_str(&format!(
            "# HELP {} {}\n",
            family.name,
            escape_help(&family.help)
        ));
        out.push_str(&format!("# TYPE {} gauge\n", family.name));
        for (labels, value) in &family.series {
            out.push_str(&family.name);
            if !labels.is_empty() {
                let rendered: Vec<_> = labels
                    .iter()
                    .map(|(name, value)| format!("{name}=\"{}\"", escape_label(value)))
                    .collect();
                out.push_str(&format!("{{{}}}", rendered.join(",")));
            }
            out.push_str(&format!(" {}\n", number(*value)));
        }
    }
    out
}

fn escape_help(text: &str) -> String {
    text.replace('\\', "\\\\").replace('\n', "\\n")
}
fn escape_label(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}
fn number(value: f64) -> String {
    if value.is_nan() {
        "NaN".into()
    } else if value.is_infinite() {
        if value > 0.0 {
            "+Inf".into()
        } else {
            "-Inf".into()
        }
    } else {
        format!("{value}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_help_type_labels_and_escapes() {
        let family = Family {
            name: "drasi_requests_overdue".into(),
            help: "Requests past their deadline.".into(),
            series: BTreeMap::from([
                (vec![("workload".into(), "support".into())], 2.0),
                (vec![("workload".into(), "a\"b\\c".into())], -1.0),
            ]),
        };
        let ready = Family {
            name: "drasi_reaction_input_ready".into(),
            help: "1 once the output's snapshot is loaded.".into(),
            series: BTreeMap::from([(vec![("output".into(), "overdue".into())], 1.0)]),
        };
        assert_eq!(
            render(&[family, ready]),
            "# HELP drasi_requests_overdue Requests past their deadline.\n\
             # TYPE drasi_requests_overdue gauge\n\
             drasi_requests_overdue{workload=\"a\\\"b\\\\c\"} -1\n\
             drasi_requests_overdue{workload=\"support\"} 2\n\
             # HELP drasi_reaction_input_ready 1 once the output's snapshot is loaded.\n\
             # TYPE drasi_reaction_input_ready gauge\n\
             drasi_reaction_input_ready{output=\"overdue\"} 1\n"
        );
    }
}
