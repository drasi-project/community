//! Mattermost, the incident channel (plan/contracts.md sections 13–14). Mattermost and its
//! buttons are real; what a button does is the existing change-approval tool:
//! - `POST /mattermost/actions`: Mattermost calls it when Approve or Reject is pressed on one of
//!   Drasi's proposal cards. It records the approval exactly as `POST /approvals` does (Drasi
//!   then applies or rejects it) and updates the card to show who decided.
//! - `POST /mattermost/approve-latest`: the desk's backup button. It presses Approve on the
//!   newest proposal card as `oncall`, so Mattermost calls `/mattermost/actions` as for a click.
//! - `GET /mattermost/sign-in`: signs the presenter's browser in as `oncall` for the projector.
//! - `clear_channel`: the demo reset deletes the channel's posts.
use crate::{external, scenario, Api, App, Error, Payload};
use anyhow::{Context, Result};
use axum::{
    extract::State,
    http::{header, HeaderMap, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

pub const PROPOSAL: &str = "proposal";

/// IDs from `.native/mattermost.json` and tokens from the environment (demo/.env).
struct Channel {
    url: String,
    team: String,
    channel: String,
    channel_id: String,
}

fn channel() -> Result<Channel> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../.native/mattermost.json");
    let ids: Value = serde_json::from_slice(
        &std::fs::read(&path).context("Mattermost isn't bootstrapped (run ops/observability-stack.sh up)")?,
    )?;
    let text = |key: &str| -> Result<String> {
        Ok(ids[key].as_str().with_context(|| format!("mattermost.json {key}"))?.to_owned())
    };
    Ok(Channel {
        url: ignite_settings::get().url("MATTERMOST")?,
        team: text("team")?,
        channel: text("channel")?,
        channel_id: text("channel_id")?,
    })
}

fn token(name: &str) -> Result<String> {
    std::env::var(name).with_context(|| format!("{name} is not set (load demo/.env)"))
}

#[derive(Deserialize)]
pub struct ActionRequest {
    #[serde(default)]
    user_name: String,
    post_id: String,
    context: ActionContext,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActionContext {
    action: String,
    candidate_id: String,
    basis: String,
    revision: i64,
    run_epoch: String,
    #[serde(default)]
    change: Option<String>,
}

/// What a button press records; `None` when the card is from an earlier run.
pub fn record_for(request: &ActionRequest, current_epoch: &str) -> std::result::Result<Option<external::Record>, Error> {
    let context = &request.context;
    let declined = match context.action.as_str() {
        "approve" => false,
        "decline" => true,
        other => return Err(Error::bad_request(&format!("unknown action {other}"))),
    };
    if request.post_id.is_empty() {
        return Err(Error::bad_request("post_id is required"));
    }
    if context.run_epoch != current_epoch {
        return Ok(None);
    }
    Ok(Some(external::Record {
        id: format!("mm-{}", request.post_id),
        candidate_id: context.candidate_id.clone(),
        basis: context.basis.clone(),
        revision: context.revision,
        declined,
        requested_by: (!request.user_name.is_empty()).then(|| format!("@{}", request.user_name)),
        post_id: Some(request.post_id.clone()),
        change: context.change.clone(),
    }))
}

/// The card after a decision: the same props without buttons, plus a Decision field.
pub fn decided_props(mut props: Value, decision: &str) -> Value {
    if let Some(attachments) = props.get_mut("attachments").and_then(Value::as_array_mut) {
        for attachment in attachments.iter_mut() {
            if let Some(object) = attachment.as_object_mut() {
                object.remove("actions");
            }
        }
        if let Some(object) = attachments.last_mut().and_then(Value::as_object_mut) {
            let fields = object.entry("fields").or_insert_with(|| json!([]));
            if let Some(fields) = fields.as_array_mut() {
                fields.push(json!({"title":"Decision","value":decision,"short":false}));
            }
        }
    }
    props
}

pub fn decision_text(request: &ActionRequest) -> String {
    let by = if request.user_name.is_empty() { "the desk".to_owned() } else { format!("@{}", request.user_name) };
    if request.context.action == "decline" {
        format!("Rejected by {by}. Nothing will change.")
    } else {
        format!("Approved by {by}. Drasi checks it still matches its current plan before anything changes.")
    }
}

/// `POST /mattermost/actions` (Mattermost's integration request for a card button).
pub async fn actions(State(app): State<App>, Payload(request): Payload<ActionRequest>) -> Api {
    let epoch = {
        let client = crate::db().await?;
        scenario::current(&client).await?.0
    };
    let Some(record) = record_for(&request, &epoch)? else {
        return Ok(Json(json!({"ephemeral_text":"This card is from an earlier run of the demo."})));
    };
    let approval = external::record(record).await?;
    let ephemeral = if request.context.action == "decline" {
        "Recorded in the change-approval tool: rejected."
    } else {
        "Sent to the change-approval tool."
    };
    // Rebuild the card from Mattermost's copy (clients never see a button's integration).
    let update = async {
        let channel = channel()?;
        let post: Value = app
            .http
            .get(format!("{}/api/v4/posts/{}", channel.url, request.post_id))
            .bearer_auth(token("MATTERMOST_ADMIN_TOKEN")?)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        anyhow::Ok(json!({
            "message": post["message"],
            "props": decided_props(post["props"].clone(), &decision_text(&request)),
        }))
    }
    .await;
    match update {
        Ok(update) => Ok(Json(json!({"update":update,"ephemeral_text":ephemeral,"approval":approval}))),
        Err(error) => {
            eprintln!("Mattermost card {} not updated: {error:#}", request.post_id);
            Ok(Json(json!({"ephemeral_text":ephemeral,"approval":approval})))
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApproveLatest {
    run_epoch: String,
}

/// The newest proposal card of this run that can still be approved, from a page of posts
/// (Mattermost's `{order, posts}`, newest first).
pub fn latest_proposal(page: &Value, run_epoch: &str) -> Option<String> {
    page["order"].as_array()?.iter().filter_map(Value::as_str).find(|id| {
        let props = &page["posts"][*id]["props"];
        props["ignite_card"] == PROPOSAL
            && props["ignite_run_epoch"] == run_epoch
            && props["attachments"].as_array().is_some_and(|a| {
                a.iter().any(|attachment| {
                    attachment["actions"].as_array().is_some_and(|actions| actions.iter().any(|x| x["id"] == "approve"))
                })
            })
    }).map(str::to_owned)
}

/// `POST /mattermost/approve-latest`: press Approve on the newest proposal card as `oncall`.
pub async fn approve_latest(State(app): State<App>, Payload(input): Payload<ApproveLatest>) -> Api {
    let unreachable = |error: anyhow::Error| Error::with(StatusCode::BAD_GATEWAY, &format!("Mattermost: {error:#}"));
    let channel = channel().map_err(unreachable)?;
    let page: Value = async {
        anyhow::Ok(
            app.http
                .get(format!("{}/api/v4/channels/{}/posts", channel.url, channel.channel_id))
                .query(&[("per_page", "100")])
                .bearer_auth(token("MATTERMOST_ADMIN_TOKEN")?)
                .send()
                .await?
                .error_for_status()?
                .json()
                .await?,
        )
    }
    .await
    .map_err(unreachable)?;
    let Some(post_id) = latest_proposal(&page, &input.run_epoch) else {
        return Err(Error::not_found(&format!("No proposed change to approve in #{}", channel.channel)));
    };
    // Mattermost calls /mattermost/actions, exactly as when the button is clicked.
    async {
        app.http
            .post(format!("{}/api/v4/posts/{post_id}/actions/approve", channel.url))
            .bearer_auth(token("MATTERMOST_ONCALL_TOKEN")?)
            .json(&json!({}))
            .send()
            .await?
            .error_for_status()?;
        anyhow::Ok(())
    }
    .await
    .map_err(unreachable)?;
    Ok(Json(json!({"post_id":post_id,"approval":format!("mm-{post_id}")})))
}

/// `GET /mattermost/sign-in`: sign in as `oncall` and hand the browser Mattermost's session
/// cookies (cookies belong to the host, not the port, so they apply to Mattermost's port too).
pub async fn sign_in(State(app): State<App>) -> Response {
    let result = async {
        let channel = channel()?;
        let response = app
            .http
            .post(format!("{}/api/v4/users/login", channel.url))
            // Mattermost sets its session cookies only for requests made like its web app's.
            .header("x-requested-with", "XMLHttpRequest")
            .json(&json!({"login_id":"oncall","password":token("MATTERMOST_ONCALL_PASSWORD")?}))
            .send()
            .await?
            .error_for_status()?;
        let mut headers = HeaderMap::new();
        for cookie in response.headers().get_all(header::SET_COOKIE) {
            headers.append(header::SET_COOKIE, cookie.clone());
        }
        let target = format!("http://localhost:{}/{}/channels/{}",
            ignite_settings::get().port("MATTERMOST")?, channel.team, channel.channel);
        headers.insert(header::LOCATION, HeaderValue::from_str(&target)?);
        anyhow::Ok(headers)
    }
    .await;
    match result {
        Ok(headers) => (StatusCode::FOUND, headers).into_response(),
        Err(error) => {
            eprintln!("Mattermost sign-in failed: {error:#}");
            (StatusCode::BAD_GATEWAY, format!("Mattermost sign-in failed: {error}")).into_response()
        }
    }
}

/// Demo driver (reset): delete every post in the incident channel.
pub async fn clear_channel(app: &App) -> Result<usize> {
    let channel = channel()?;
    let admin = token("MATTERMOST_ADMIN_TOKEN")?;
    let mut removed = 0;
    loop {
        let page: Value = app
            .http
            .get(format!("{}/api/v4/channels/{}/posts", channel.url, channel.channel_id))
            .query(&[("per_page", "200")])
            .bearer_auth(&admin)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;
        let order: Vec<String> = page["order"].as_array().into_iter().flatten()
            .filter_map(|id| id.as_str().map(str::to_owned)).collect();
        if order.is_empty() {
            return Ok(removed);
        }
        for id in order {
            app.http
                .delete(format!("{}/api/v4/posts/{id}", channel.url))
                .bearer_auth(&admin)
                .send()
                .await?
                .error_for_status()?;
            removed += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(action: &str, epoch: &str) -> ActionRequest {
        serde_json::from_value(json!({
            "user_id":"u","user_name":"oncall","channel_id":"c","channel_name":"incident-ai-support",
            "team_id":"t","team_domain":"ai-platform","post_id":"p1","trigger_id":"x","type":"","data_source":"",
            "context":{"action":action,"candidate_id":"cand","basis":"b","revision":7,"run_epoch":epoch,
                       "change":"Move summaries to US-C and support to EU-B"}
        }))
        .unwrap()
    }

    #[test]
    fn approve_is_the_approval_tools_request() {
        let record = record_for(&request("approve", "run-1"), "run-1").ok().unwrap().unwrap();
        assert_eq!(record.id, "mm-p1");
        assert_eq!((record.candidate_id.as_str(), record.basis.as_str(), record.revision), ("cand", "b", 7));
        assert!(!record.declined);
        assert_eq!(record.requested_by.as_deref(), Some("@oncall"));
        assert_eq!(record.post_id.as_deref(), Some("p1"));
        assert_eq!(record.change.as_deref(), Some("Move summaries to US-C and support to EU-B"));
    }

    #[test]
    fn decline_is_recorded_as_declined() {
        let record = record_for(&request("decline", "run-1"), "run-1").ok().unwrap().unwrap();
        assert!(record.declined);
        assert!(decision_text(&request("decline", "run-1")).starts_with("Rejected by @oncall"));
    }

    #[test]
    fn earlier_run_and_unknown_actions() {
        assert!(record_for(&request("approve", "run-0"), "run-1").ok().unwrap().is_none());
        assert!(record_for(&request("launch", "run-1"), "run-1").is_err());
    }

    #[test]
    fn decided_card_loses_its_buttons() {
        let props = json!({"ignite_card":"proposal","attachments":[{"title":"Proposed change","fields":[{"title":"Why","value":"EU-A is down"}],
            "actions":[{"id":"approve","name":"Approve"},{"id":"decline","name":"Reject"}]}]});
        let decided = decided_props(props, "Approved by @oncall.");
        let attachment = &decided["attachments"][0];
        assert!(attachment.get("actions").is_none());
        assert_eq!(attachment["fields"][1]["title"], "Decision");
        assert_eq!(decided["ignite_card"], "proposal");
    }

    #[test]
    fn latest_proposal_is_the_newest_open_card_of_this_run() {
        let card = |epoch: &str, open: bool| json!({"props":{"ignite_card":"proposal","ignite_run_epoch":epoch,
            "attachments":[if open { json!({"actions":[{"id":"approve"}]}) } else { json!({}) }]}});
        let page = json!({"order":["new-decided","old-run","open","older-open"],"posts":{
            "new-decided":card("run-1", false),"old-run":card("run-0", true),
            "open":card("run-1", true),"older-open":card("run-1", true)}});
        assert_eq!(latest_proposal(&page, "run-1").as_deref(), Some("open"));
        assert_eq!(latest_proposal(&page, "run-2"), None);
    }
}
