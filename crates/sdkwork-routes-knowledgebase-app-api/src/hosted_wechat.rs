use std::sync::Arc;

use async_trait::async_trait;

use sdkwork_intelligence_knowledgebase_service::wechat::{
    KnowledgeWechatService, WechatApiClient, WechatCallbackReceipt,
    WechatCallbackReceiptRequest, WechatCallbackService, WechatCallbackVerificationRequest,
};
use sdkwork_knowledgebase_contract::wechat::{
    KnowledgeWechatAppletList, KnowledgeWechatArticlesPreviewRequest,
    KnowledgeWechatArticlesPublishRequest, KnowledgeWechatFanTagList,
    KnowledgeWechatOfficialAccountList, KnowledgeWechatOperationResult,
    KnowledgeWechatReplaceAppletsRequest, KnowledgeWechatReplaceOfficialAccountsRequest,
};

use crate::{
    hosted_access::ensure_runtime_tenant, runtime::KnowledgebaseRuntime, ApiError, ApiResult,
    KnowledgeAppRequestContext, KnowledgeWechatAppService,
};

#[derive(Clone)]
pub(crate) struct HostedWechatService {
    runtime: KnowledgebaseRuntime,
    /// One client for the service's lifetime. `HostedWechatService` is built once with the app
    /// router, so this is where the process-wide WeChat access token cache actually lives; a
    /// per-request client would re-fetch a token for every call and exhaust the WeChat quota.
    api_client: Arc<WechatApiClient>,
}

impl HostedWechatService {
    pub fn new(runtime: KnowledgebaseRuntime) -> Self {
        Self {
            runtime,
            api_client: Arc::new(WechatApiClient::new()),
        }
    }

    fn service(&self) -> KnowledgeWechatService<'_> {
        KnowledgeWechatService::new(
            self.runtime.drive_storage(),
            self.runtime.tenant_id_str(),
            Arc::clone(&self.api_client),
        )
    }

    fn callback_service(&self) -> WechatCallbackService<'_> {
        WechatCallbackService::new(self.runtime.drive_storage(), self.runtime.tenant_id_str())
    }

    /// WeChat callers are unauthenticated, so the tenant query parameter is checked
    /// against the deployment's runtime tenant before anything else runs.
    fn ensure_callback_tenant(&self, tenant_id: u64) -> Result<(), ApiError> {
        if tenant_id != self.runtime.tenant_id() {
            return Err(ApiError::invalid_request(
                "invalid_wechat_callback",
                "callback tenant is not served by this runtime",
            ));
        }
        Ok(())
    }
}

#[async_trait]
impl KnowledgeWechatAppService for HostedWechatService {
    async fn list_official_accounts(
        &self,
        context: KnowledgeAppRequestContext,
    ) -> ApiResult<KnowledgeWechatOfficialAccountList> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        let accounts = self
            .service()
            .list_official_accounts()
            .await
            .map_err(ApiError::from)?;
        Ok(KnowledgeWechatOfficialAccountList { accounts })
    }

    async fn replace_official_accounts(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatReplaceOfficialAccountsRequest,
    ) -> ApiResult<KnowledgeWechatOfficialAccountList> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        let accounts = self
            .service()
            .replace_official_accounts(request.accounts)
            .await
            .map_err(ApiError::from)?;
        Ok(KnowledgeWechatOfficialAccountList { accounts })
    }

    async fn list_official_account_fan_tags(
        &self,
        context: KnowledgeAppRequestContext,
        account_id: String,
    ) -> ApiResult<KnowledgeWechatFanTagList> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        self.service()
            .list_fan_tags(&account_id)
            .await
            .map_err(ApiError::from)
    }

    async fn list_applets(
        &self,
        context: KnowledgeAppRequestContext,
    ) -> ApiResult<KnowledgeWechatAppletList> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        let applets = self
            .service()
            .list_applets()
            .await
            .map_err(ApiError::from)?;
        Ok(KnowledgeWechatAppletList { applets })
    }

    async fn replace_applets(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatReplaceAppletsRequest,
    ) -> ApiResult<KnowledgeWechatAppletList> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        let applets = self
            .service()
            .replace_applets(request.applets)
            .await
            .map_err(ApiError::from)?;
        Ok(KnowledgeWechatAppletList { applets })
    }

    async fn publish_articles(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatArticlesPublishRequest,
    ) -> ApiResult<KnowledgeWechatOperationResult> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        self.service()
            .publish_articles(request)
            .await
            .map_err(ApiError::from)
    }

    async fn preview_articles(
        &self,
        context: KnowledgeAppRequestContext,
        request: KnowledgeWechatArticlesPreviewRequest,
    ) -> ApiResult<KnowledgeWechatOperationResult> {
        ensure_runtime_tenant(&self.runtime, &context)?;
        self.service()
            .preview_articles(request)
            .await
            .map_err(ApiError::from)
    }

    async fn verify_wechat_callback(
        &self,
        request: WechatCallbackVerificationRequest,
    ) -> ApiResult<String> {
        self.ensure_callback_tenant(request.tenant_id)?;
        self.callback_service()
            .verify_url(&request)
            .await
            .map_err(ApiError::from)
    }

    async fn receive_wechat_callback(
        &self,
        request: WechatCallbackReceiptRequest,
    ) -> ApiResult<WechatCallbackReceipt> {
        self.ensure_callback_tenant(request.tenant_id)?;
        self.callback_service()
            .receive_message(&request)
            .await
            .map_err(ApiError::from)
    }
}
