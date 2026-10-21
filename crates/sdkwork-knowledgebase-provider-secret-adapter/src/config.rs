use std::path::PathBuf;
use std::time::Duration;

use thiserror::Error;

pub const KNOWLEDGEBASE_PROVIDER_SECRET_ENV_PREFIX: &str = "SDKWORK_KNOWLEDGEBASE_PROVIDER_SECRET_";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KnowledgebaseProviderCredentialEnvironment {
    Development,
    Test,
    Staging,
    Production,
}

impl KnowledgebaseProviderCredentialEnvironment {
    pub fn parse(
        value: &str,
    ) -> Result<Self, KnowledgebaseProviderCredentialResolverConfigurationError> {
        match value.trim().to_ascii_lowercase().as_str() {
            "development" => Ok(Self::Development),
            "test" => Ok(Self::Test),
            "staging" => Ok(Self::Staging),
            "production" => Ok(Self::Production),
            _ => Err(KnowledgebaseProviderCredentialResolverConfigurationError::InvalidEnvironment),
        }
    }

    pub fn allows_local_sources(self) -> bool {
        matches!(self, Self::Development | Self::Test)
    }

    pub fn requires_managed_source(self) -> bool {
        matches!(self, Self::Staging | Self::Production)
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Development => "development",
            Self::Test => "test",
            Self::Staging => "staging",
            Self::Production => "production",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnowledgebaseProviderCredentialResolverConfig {
    pub(crate) environment: KnowledgebaseProviderCredentialEnvironment,
    pub(crate) local_secret_root: Option<PathBuf>,
    pub(crate) max_credential_bytes: usize,
    pub(crate) max_managed_resolution_duration: Duration,
    pub(crate) max_managed_concurrency: usize,
}

impl KnowledgebaseProviderCredentialResolverConfig {
    pub const DEFAULT_MAX_CREDENTIAL_BYTES: usize = 64 * 1024;
    pub const DEFAULT_MAX_MANAGED_RESOLUTION_DURATION: Duration = Duration::from_secs(5);
    pub const MAX_MANAGED_RESOLUTION_DURATION: Duration = Duration::from_secs(30);
    pub const DEFAULT_MAX_MANAGED_CONCURRENCY: usize = 32;
    pub const MAX_MANAGED_CONCURRENCY: usize = 256;

    pub fn local(
        environment: KnowledgebaseProviderCredentialEnvironment,
        local_secret_root: Option<PathBuf>,
    ) -> Result<Self, KnowledgebaseProviderCredentialResolverConfigurationError> {
        if !environment.allows_local_sources() {
            return Err(
                KnowledgebaseProviderCredentialResolverConfigurationError::ManagedSourceRequired,
            );
        }
        if local_secret_root
            .as_ref()
            .is_some_and(|root| !root.is_absolute())
        {
            return Err(
                KnowledgebaseProviderCredentialResolverConfigurationError::SecretRootMustBeAbsolute,
            );
        }
        Ok(Self {
            environment,
            local_secret_root,
            max_credential_bytes: Self::DEFAULT_MAX_CREDENTIAL_BYTES,
            max_managed_resolution_duration: Self::DEFAULT_MAX_MANAGED_RESOLUTION_DURATION,
            max_managed_concurrency: Self::DEFAULT_MAX_MANAGED_CONCURRENCY,
        })
    }

    pub fn managed(
        environment: KnowledgebaseProviderCredentialEnvironment,
    ) -> Result<Self, KnowledgebaseProviderCredentialResolverConfigurationError> {
        if !environment.requires_managed_source() {
            return Err(
                KnowledgebaseProviderCredentialResolverConfigurationError::LocalSourceRequired,
            );
        }
        Ok(Self {
            environment,
            local_secret_root: None,
            max_credential_bytes: Self::DEFAULT_MAX_CREDENTIAL_BYTES,
            max_managed_resolution_duration: Self::DEFAULT_MAX_MANAGED_RESOLUTION_DURATION,
            max_managed_concurrency: Self::DEFAULT_MAX_MANAGED_CONCURRENCY,
        })
    }

    pub fn with_max_credential_bytes(
        mut self,
        max_credential_bytes: usize,
    ) -> Result<Self, KnowledgebaseProviderCredentialResolverConfigurationError> {
        if max_credential_bytes == 0 || max_credential_bytes > Self::DEFAULT_MAX_CREDENTIAL_BYTES {
            return Err(
                KnowledgebaseProviderCredentialResolverConfigurationError::InvalidCredentialSizeLimit,
            );
        }
        self.max_credential_bytes = max_credential_bytes;
        Ok(self)
    }

    pub fn with_max_managed_resolution_duration(
        mut self,
        duration: Duration,
    ) -> Result<Self, KnowledgebaseProviderCredentialResolverConfigurationError> {
        if duration.is_zero() || duration > Self::MAX_MANAGED_RESOLUTION_DURATION {
            return Err(
                KnowledgebaseProviderCredentialResolverConfigurationError::InvalidManagedResolutionTimeout,
            );
        }
        self.max_managed_resolution_duration = duration;
        Ok(self)
    }

    pub fn with_max_managed_concurrency(
        mut self,
        max_managed_concurrency: usize,
    ) -> Result<Self, KnowledgebaseProviderCredentialResolverConfigurationError> {
        if max_managed_concurrency == 0 || max_managed_concurrency > Self::MAX_MANAGED_CONCURRENCY {
            return Err(
                KnowledgebaseProviderCredentialResolverConfigurationError::InvalidManagedConcurrency,
            );
        }
        self.max_managed_concurrency = max_managed_concurrency;
        Ok(self)
    }
}

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum KnowledgebaseProviderCredentialResolverConfigurationError {
    #[error("Knowledgebase Provider credential environment is invalid")]
    InvalidEnvironment,
    #[error("Knowledgebase Provider credentials require a managed Secret Provider")]
    ManagedSourceRequired,
    #[error("Knowledgebase Provider credentials require a local source policy")]
    LocalSourceRequired,
    #[error("Knowledgebase Provider credential secret root must be absolute")]
    SecretRootMustBeAbsolute,
    #[error("Knowledgebase Provider credential size limit is invalid")]
    InvalidCredentialSizeLimit,
    #[error("Knowledgebase Provider managed credential resolution timeout is invalid")]
    InvalidManagedResolutionTimeout,
    #[error("Knowledgebase Provider managed credential concurrency limit is invalid")]
    InvalidManagedConcurrency,
}
