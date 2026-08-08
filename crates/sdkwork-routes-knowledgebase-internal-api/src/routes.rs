use axum::{
    routing::{get, post},
    Router,
};
use std::sync::Arc;

use crate::{
    handlers::{
        list_wiki_navigation, receive_drive_event, resolve_wiki_route, retrieve_wiki_content,
        retrieve_wiki_publication, search_wiki_pages,
    },
    state::{InternalApiState, KnowledgebaseDriveEventReceiver, KnowledgebaseWikiPublicProvider},
};

fn business_router(state: InternalApiState) -> Router {
    Router::new()
        .route(
            "/internal/v3/api/knowledgebase/drive_events",
            post(receive_drive_event),
        )
        .route(
            "/internal/v3/api/knowledgebase/wiki_publications/{publicationUuid}",
            get(retrieve_wiki_publication),
        )
        .route(
            "/internal/v3/api/knowledgebase/wiki_publications/{publicationUuid}/routes/resolve",
            post(resolve_wiki_route),
        )
        .route(
            "/internal/v3/api/knowledgebase/wiki_publications/{publicationUuid}/contents/{contentHandle}",
            get(retrieve_wiki_content),
        )
        .route(
            "/internal/v3/api/knowledgebase/wiki_publications/{publicationUuid}/navigation",
            get(list_wiki_navigation),
        )
        .route(
            "/internal/v3/api/knowledgebase/wiki_publications/{publicationUuid}/pages/search",
            get(search_wiki_pages),
        )
        // Internal wiki/Drive event payloads are bounded to 64 KiB — deliberately stricter
        // than the 1 MiB transport limit on the app/backend/open surfaces — because every
        // internal callback body is verified, HMAC-signed, and processed synchronously;
        // oversized payloads must fail before signature verification.
        .layer(axum::extract::DefaultBodyLimit::max(65_536))
        .with_state(state)
}

pub fn build_business_router_with_services(
    receiver: Arc<dyn KnowledgebaseDriveEventReceiver>,
    wiki_provider: Arc<dyn KnowledgebaseWikiPublicProvider>,
    drive_event_caller_app_id: impl Into<String>,
    wiki_provider_caller_app_id: impl Into<String>,
) -> Router {
    business_router(InternalApiState::new(
        receiver,
        wiki_provider,
        drive_event_caller_app_id,
        wiki_provider_caller_app_id,
    ))
}

pub fn build_router_with_services(
    receiver: Arc<dyn KnowledgebaseDriveEventReceiver>,
    wiki_provider: Arc<dyn KnowledgebaseWikiPublicProvider>,
    drive_event_caller_app_id: impl Into<String>,
    wiki_provider_caller_app_id: impl Into<String>,
) -> Router {
    let router = build_business_router_with_services(
        receiver,
        wiki_provider,
        drive_event_caller_app_id,
        wiki_provider_caller_app_id,
    );
    crate::web_bootstrap::wrap_with_default_resolver(router)
}

pub fn gateway_mount(
    receiver: Arc<dyn KnowledgebaseDriveEventReceiver>,
    wiki_provider: Arc<dyn KnowledgebaseWikiPublicProvider>,
    drive_event_caller_app_id: impl Into<String>,
    wiki_provider_caller_app_id: impl Into<String>,
) -> Router {
    build_router_with_services(
        receiver,
        wiki_provider,
        drive_event_caller_app_id,
        wiki_provider_caller_app_id,
    )
}

pub async fn wrap_router_with_web_framework_from_env(
    receiver: Arc<dyn KnowledgebaseDriveEventReceiver>,
    wiki_provider: Arc<dyn KnowledgebaseWikiPublicProvider>,
    drive_event_caller_app_id: impl Into<String>,
    wiki_provider_caller_app_id: impl Into<String>,
) -> Router {
    let router = build_business_router_with_services(
        receiver,
        wiki_provider,
        drive_event_caller_app_id,
        wiki_provider_caller_app_id,
    );
    crate::web_bootstrap::wrap_from_env(router).await
}
