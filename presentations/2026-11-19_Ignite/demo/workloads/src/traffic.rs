use super::*;

#[derive(Default)]
pub struct Traffic {
    epoch: Option<String>,
    until_ms: i64,
    submitted: u64,
    error: Option<String>,
}

impl Traffic {
    pub fn snapshot(&self) -> Value {
        json!({
            "running":self.epoch.is_some() && self.until_ms > now(),
            "submitted":self.submitted, "ends_ms":self.until_ms, "error":self.error,
            "interval_seconds":5
        })
    }

    fn stop(&mut self, reason: Option<String>) {
        self.epoch = None;
        self.error = reason;
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    run_epoch: String,
    running: bool,
}

pub async fn control(State(app): State<App>, Payload(input): Payload<Input>) -> Api {
    let (epoch, _) = scenario::current(&db().await?).await?;
    let mut traffic = app.traffic.lock().await;
    if input.running {
        if epoch != input.run_epoch {
            return Err(Error::conflict("stale traffic scenario"));
        }
        if traffic.epoch.as_ref() != Some(&epoch) || traffic.until_ms <= now() {
            *traffic = Traffic {
                epoch: Some(epoch),
                until_ms: now() + 90 * 60 * 1000,
                submitted: 0,
                error: None,
            };
        }
    } else {
        traffic.stop(None);
    }
    Ok(Json(traffic.snapshot()))
}

pub async fn tick(app: &App) {
    let mut traffic = app.traffic.lock().await;
    let Some(epoch) = traffic.epoch.clone() else {
        return;
    };
    if traffic.until_ms <= now() {
        traffic.stop(None);
        return;
    }
    let result = async {
        let (current, _) = scenario::current(&db().await?).await?;
        anyhow::ensure!(current == epoch, "Scenario changed; start traffic again");
        for workload in ["support", "batch"] {
            let _response = submit(
                State(app.clone()),
                Payload(Submit {
                    id: format!("{epoch}-traffic-{workload}-{}", identity()),
                    run_epoch: epoch.clone(),
                    workload: workload.into(),
                    content: crate::content::question(workload, traffic.submitted as usize / 2)
                        .into(),
                    destination: None,
                }),
            )
            .await
            .map_err(|e| e.error)?;
            traffic.submitted += 1;
        }
        Ok::<(), anyhow::Error>(())
    }
    .await;
    if let Err(error) = result {
        eprintln!("Demo traffic stopped: {error:#}");
        traffic.stop(Some(error.to_string()));
    }
}
