//! Deployment metadata and admin route composition. Typed host control lives in `cloud::control`.
use super::*;
use super::changes::{SpaceChanges, ChangeQuery};
use axum::extract::{
    Path as Param, Query,
    rejection::{JsonRejection, PathRejection, QueryRejection},
};
use communication::{
    HostView,
    types::{CloudAgentSummary, MessageHistoryPage, MessageHistoryQuery, MessageView, SendHumanMessage, TranscriptHistoryPage, TranscriptHistoryQuery},
};
use utoipa::OpenApi;

#[derive(Serialize, ToSchema)]
pub(super) struct ScopeReceipt {
    pub(super) id: String,
    pub(super) scope_url: String,
    pub(super) credential_version: u32,
    pub(super) generation: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct CreateScope {
    #[schema(min_length = 1, max_length = 128)]
    request_id: String,
    #[schema(min_length = 32, max_length = 256, pattern = "^[A-Za-z0-9_-]+$", write_only = true)]
    token: String,
}
#[derive(Deserialize, ToSchema)]
#[serde(deny_unknown_fields)]
struct RotateToken {
    #[schema(min_length = 1, max_length = 128)]
    request_id: String,
    expected_version: u32,
    #[schema(min_length = 32, max_length = 256, pattern = "^[A-Za-z0-9_-]+$", write_only = true)]
    token: String,
}
#[derive(Serialize, ToSchema)]
struct ScopeRow {
    id: String,
    scope_url: String,
    credential_version: u32,
    generation: String,
    created_at: i64,
    hosts: u32,
    online_hosts: u32,
    agents: u32,
    available: bool,
}
#[derive(Serialize, ToSchema, Default)]
struct AdminOverview {
    deployment_id: String,
    protocol_version: u32,
    observed_at: i64,
    scopes: u32,
    hosts: u32,
    online_hosts: u32,
    agents: u32,
    unavailable: u32,
}
#[derive(Serialize, ToSchema)]
struct ScopePage {
    items: Vec<ScopeRow>,
    #[schema(required = true)]
    next: Option<String>,
}
#[derive(Serialize, ToSchema)]
struct HostPage {
    items: Vec<HostView>,
    #[schema(required = true)]
    next: Option<String>,
}
#[derive(Serialize, ToSchema)]
struct AgentPage {
    items: Vec<CloudAgentSummary>,
    #[schema(required = true)]
    next: Option<String>,
}
#[derive(Deserialize, Default, utoipa::IntoParams)]
#[into_params(parameter_in=Query)]
#[serde(deny_unknown_fields)]
struct PageQuery {
    after: Option<String>,
    limit: Option<u32>,
}
impl PageQuery {
    fn parse(raw: Result<Query<Self>, QueryRejection>) -> Result<(String, usize), Failure> {
        let Query(p) = raw.map_err(|_| Failure::invalid("invalid page query"))?;
        let limit = p.limit.unwrap_or(50);
        let after = p.after.unwrap_or_default();
        if !(1..=100).contains(&limit) || after.len() > 256 {
            return Err(Failure::invalid("invalid limit or cursor"));
        }
        Ok((after, limit as usize))
    }
}
fn paginate<T>(mut items: Vec<T>, limit: usize, key: impl Fn(&T) -> &str) -> (Vec<T>, Option<String>) {
    let more = items.len() > limit;
    items.truncate(limit);
    let next = if more { items.last().map(|v| key(v).to_owned()) } else { None };
    (items, next)
}

type ScopeMeta = (ScopeReceipt, PathBuf, i64);
async fn scope_meta(state: &Deployment, id: String) -> Result<ScopeMeta, Failure> {
    state.catalog(move |db| Ok((db.lookup(&id)?, db.root_for(&id), db.created_at(&id)?))).await
}
async fn metadata_page(state: &Deployment, after: String, limit: usize) -> Result<Vec<ScopeMeta>, Failure> {
    state
        .catalog(move |db| {
            db.list(&after, limit as i64)?
                .into_iter()
                .map(|r| {
                    let root = db.root_for(&r.id);
                    let created = db.created_at(&r.id)?;
                    Ok((r, root, created))
                })
                .collect()
        })
        .await
}
async fn read_scope<T: Send + 'static>(read: impl FnOnce() -> anyhow::Result<T> + Send + 'static) -> Result<T, Failure> {
    tokio::task::spawn_blocking(read).await.map_err(|_| Failure::internal())?.map_err(communication::metadata_failure)
}
async fn summarize((r, root, created_at): ScopeMeta) -> ScopeRow {
    let generation = r.generation.clone();
    let counts = read_scope(move || communication::summary(&root, &generation)).await;
    let available = counts.is_ok();
    let (hosts, online_hosts, agents) = counts.unwrap_or_default();
    ScopeRow { id: r.id, scope_url: r.scope_url, credential_version: r.credential_version, generation: r.generation, created_at, online_hosts, hosts, agents, available }
}

#[utoipa::path(get,path="/api/v1/admin/overview",operation_id="getAdminOverview",responses((status=200,body=AdminOverview)))]
async fn overview(State(state): State<Deployment>) -> Result<Json<AdminOverview>, Failure> {
    let deployment_id = state.catalog(|db| Ok(db.deployment_id.clone())).await?;
    let mut result = AdminOverview { deployment_id, protocol_version: 1, observed_at: chrono::Utc::now().timestamp(), ..Default::default() };
    let mut after = String::new();
    loop {
        let batch = metadata_page(&state, after, 100).await?;
        if batch.is_empty() {
            break;
        }
        after = batch.last().unwrap().0.id.clone();
        let done = batch.len() < 100;
        for meta in batch {
            let s = summarize(meta).await;
            result.scopes += 1;
            result.hosts += s.hosts;
            result.online_hosts += s.online_hosts;
            result.agents += s.agents;
            if !s.available {
                result.unavailable += 1
            }
        }
        if done {
            break;
        }
    }
    Ok(Json(result))
}
#[utoipa::path(get,path="/api/v1/admin/scopes",operation_id="listAdminScopes",params(PageQuery),responses((status=200,body=ScopePage)))]
async fn list_scopes(State(state): State<Deployment>, query: Result<Query<PageQuery>, QueryRejection>) -> Result<Json<ScopePage>, Failure> {
    let (after, limit) = PageQuery::parse(query)?;
    let (batch, next) = paginate(metadata_page(&state, after, limit + 1).await?, limit, |v| &v.0.id);
    let mut items = Vec::with_capacity(batch.len());
    for scope in batch {
        items.push(summarize(scope).await)
    }
    Ok(Json(ScopePage { items, next }))
}
#[utoipa::path(get,path="/api/v1/admin/scopes/{id}",operation_id="getAdminScope",params(("id"=String,Path)),responses((status=200,body=ScopeRow)))]
async fn show(State(state): State<Deployment>, Param(id): Param<String>) -> Result<Json<ScopeRow>, Failure> {
    Ok(Json(summarize(scope_meta(&state, id).await?).await))
}
#[utoipa::path(get,path="/api/v1/admin/scopes/{id}/hosts",operation_id="listAdminScopeHosts",params(("id"=String,Path),PageQuery),responses((status=200,body=HostPage)))]
async fn hosts(State(state): State<Deployment>, Param(id): Param<String>, query: Result<Query<PageQuery>, QueryRejection>) -> Result<Json<HostPage>, Failure> {
    let (after, limit) = PageQuery::parse(query)?;
    let (scope, root, _) = scope_meta(&state, id).await?;
    let cursor = (!after.is_empty()).then_some(after);
    let page = read_scope(move || communication::host_page(&root, &scope.generation, cursor, limit as u32)).await?;
    Ok(Json(HostPage { items: page.items, next: page.next }))
}
#[derive(Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in=Query)]
#[serde(deny_unknown_fields)]
struct AgentQuery {
    after: Option<String>,
    limit: Option<u32>,
    host: Option<String>,
    facts: Option<bool>,
}

#[utoipa::path(get,path="/api/v1/admin/scopes/{id}/agents",operation_id="listAdminScopeAgents",params(("id"=String,Path),AgentQuery),responses((status=200,body=AgentPage)))]
async fn agents(State(state): State<Deployment>, Param(id): Param<String>, query: Result<Query<AgentQuery>, QueryRejection>) -> Result<Json<AgentPage>, Failure> {
    let Query(q) = query.map_err(|_| Failure::invalid("invalid agent page query"))?;
    let (after, limit) = PageQuery::parse(Ok(Query(PageQuery { after: q.after, limit: q.limit })))?;
    let (scope, root, _) = scope_meta(&state, id).await?;
    let cursor = (!after.is_empty()).then_some(after);
    let page = read_scope(move || communication::agent_page(&root, &scope.generation, cursor, limit as u32, q.host, q.facts)).await?;
    Ok(Json(AgentPage { items: page.items, next: page.next }))
}

pub(super) enum AdminError {
    Deployment(Failure),
    Scope(communication::store::Error),
}

impl IntoResponse for AdminError {
    fn into_response(self) -> Response {
        match self {
            Self::Deployment(error) => error.into_response(),
            Self::Scope(error) => error.into_response(),
        }
    }
}

pub(super) async fn scope_context(state: Deployment, id: String) -> Result<ScopeContext, AdminError> {
    state
        .catalog(move |db| {
            let receipt = db.lookup(&id)?;
            Ok(db.context(&id, receipt.generation, receipt.credential_version))
        })
        .await
        .map_err(AdminError::Deployment)
}

#[utoipa::path(get,path="/api/v1/admin/scopes/{id}/changes",operation_id="adminSpaceChanges",params(("id"=String,Path),ChangeQuery),responses((status=200,body=SpaceChanges)))]
async fn admin_changes(State(state): State<Deployment>, Param(id): Param<String>, input: Result<Query<ChangeQuery>, QueryRejection>) -> Result<Json<SpaceChanges>, AdminError> {
    let query = communication::query(input).map_err(AdminError::Scope)?;
    changes::wait(scope_context(state, id).await?, query, false).await.map(Json).map_err(AdminError::Scope)
}

#[utoipa::path(get,path="/api/v1/admin/scopes/{id}/messages",operation_id="listAdminScopeMessages",params(("id"=String,Path),MessageHistoryQuery),responses((status=200,body=MessageHistoryPage)))]
async fn list_messages(State(state): State<Deployment>, id: Result<Param<String>, PathRejection>, query: Result<Query<MessageHistoryQuery>, QueryRejection>) -> Result<Json<MessageHistoryPage>, AdminError> {
    let query = communication::query(query).map_err(AdminError::Scope)?;
    let scope = scope_context(state, communication::path(id).map_err(AdminError::Scope)?).await?;
    communication::admin_message_history(scope, query).await.map(Json).map_err(AdminError::Scope)
}

#[utoipa::path(get,path="/api/v1/admin/scopes/{id}/transcript",operation_id="listAdminScopeTranscript",params(("id"=String,Path),TranscriptHistoryQuery),responses((status=200,body=TranscriptHistoryPage)))]
async fn list_transcript(State(state): State<Deployment>, id: Result<Param<String>, PathRejection>, query: Result<Query<TranscriptHistoryQuery>, QueryRejection>) -> Result<Json<TranscriptHistoryPage>, AdminError> {
    let query = communication::query(query).map_err(AdminError::Scope)?;
    let scope = scope_context(state, communication::path(id).map_err(AdminError::Scope)?).await?;
    communication::admin_transcript_history(scope, query).await.map(Json).map_err(AdminError::Scope)
}

#[utoipa::path(post,path="/api/v1/admin/scopes/{id}/messages",operation_id="sendAdminScopeMessage",params(("id"=String,Path)),request_body=SendHumanMessage,responses((status=200,body=MessageView),(status=413,description="Payload too large",body=ApiError)))]
async fn send_message(State(state): State<Deployment>, id: Result<Param<String>, PathRejection>, input: Result<Json<SendHumanMessage>, JsonRejection>) -> Result<Json<MessageView>, AdminError> {
    let input = communication::payload(input).map_err(AdminError::Scope)?;
    let scope = scope_context(state, communication::path(id).map_err(AdminError::Scope)?).await?;
    communication::admin_send_human_message(scope, input).await.map(Json).map_err(AdminError::Scope)
}

async fn change(state: Deployment, id: String, request: String, token: String, expected: Option<u32>) -> Result<Json<ScopeReceipt>, Failure> {
    if catalog::matches(&catalog::hash(&token), &state.admin_hash) {
        return Err(Failure::invalid("scope and deployment credentials must differ"));
    }
    state.catalog(move |db| db.change(&id, &request, &token, expected)).await.map(Json)
}
#[utoipa::path(put,path="/api/v1/admin/scopes/{id}",operation_id="putAdminScope",params(("id"=String,Path)),request_body=CreateScope,responses((status=200,body=ScopeReceipt)))]
async fn create(State(state): State<Deployment>, Param(id): Param<String>, body: Result<Json<CreateScope>, JsonRejection>) -> Result<Json<ScopeReceipt>, Failure> {
    let Json(b) = body.map_err(|_| Failure::invalid("invalid create request"))?;
    change(state, id, b.request_id, b.token, None).await
}
#[utoipa::path(post,path="/api/v1/admin/scopes/{id}/token-rotations",operation_id="rotateAdminScopeToken",params(("id"=String,Path)),request_body=RotateToken,responses((status=200,body=ScopeReceipt)))]
async fn rotate(State(state): State<Deployment>, Param(id): Param<String>, body: Result<Json<RotateToken>, JsonRejection>) -> Result<Json<ScopeReceipt>, Failure> {
    let Json(b) = body.map_err(|_| Failure::invalid("invalid rotation request"))?;
    change(state, id, b.request_id, b.token, Some(b.expected_version)).await
}

pub(super) fn router(state: Deployment) -> Router {
    Router::new()
        .route("/scopes/{id}/changes", get(admin_changes))
        .route("/overview", get(overview))
        .route("/scopes", get(list_scopes))
        .route("/scopes/{id}", get(show).put(create))
        .route("/scopes/{id}/hosts", get(hosts))
        .route("/scopes/{id}/agents", get(agents))
        .route("/scopes/{id}/transcript", get(list_transcript))
        .route("/scopes/{id}/messages", get(list_messages).post(send_message).layer(DefaultBodyLimit::max(512 * 1024)))
        .route("/scopes/{id}/token-rotations", axum::routing::post(rotate))
        .merge(super::control::admin_router())
        .with_state(state)
}

pub(super) fn document() -> serde_json::Value {
    #[derive(OpenApi)]
    #[openapi(paths(admin_changes, overview, list_scopes, show, hosts, agents, list_transcript, list_messages, send_message, create, rotate), components(schemas(ApiError)))]
    struct AdminApi;
    let mut api = AdminApi::openapi();
    api.merge(super::control::admin_document());
    let mut doc = serde_json::to_value(api).expect("serializable OpenAPI");
    doc["info"]["title"] = "AgentX Deployment Administration".into();
    doc["components"]["securitySchemes"]["adminBearer"] = serde_json::json!({"type":"http","scheme":"bearer"});
    doc["security"] = serde_json::json!([{"adminBearer":[]}]);
    for path in doc["paths"].as_object_mut().unwrap().values_mut() {
        for op in path.as_object_mut().unwrap().values_mut() {
            for (code, reason) in [("400", "Invalid input"), ("401", "Invalid admin credential"), ("404", "Unknown scope"), ("409", "Conflict"), ("500", "Storage error"), ("503", "Unavailable")] {
                op["responses"][code] = serde_json::json!({"description":reason,"content":{"application/json":{"schema":{"$ref":"#/components/schemas/ApiError"}}}});
            }
        }
    }
    doc
}
