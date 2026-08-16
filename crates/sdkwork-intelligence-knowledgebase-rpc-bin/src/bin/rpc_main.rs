#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(request) = information_request(std::env::args().skip(1)) {
        print_information(request);
        return Ok(());
    }
    install_process_crypto_provider()?;
    sdkwork_database_sqlx::enable_process_shared_database_pool();
    sdkwork_intelligence_knowledgebase_rpc_bin::run_group_knowledge_space_lifecycle_rpc_from_env()
        .await
        .map_err(|error| Box::new(error) as Box<dyn std::error::Error + Send + Sync>)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InformationRequest {
    Help,
    Version,
}

fn information_request(args: impl IntoIterator<Item = String>) -> Option<InformationRequest> {
    args.into_iter()
        .find_map(|argument| match argument.as_str() {
            "-h" | "--help" => Some(InformationRequest::Help),
            "-V" | "--version" => Some(InformationRequest::Version),
            _ => None,
        })
}

fn print_information(request: InformationRequest) {
    match request {
        InformationRequest::Help => println!(
            "sdkwork-intelligence-knowledgebase-rpc-bin {}\n\n\
             Internal Knowledgebase RPC service host.\n\n\
             Usage: sdkwork-intelligence-knowledgebase-rpc-bin [OPTIONS]\n\n\
             Options:\n  -h, --help     Print help\n  -V, --version  Print version",
            env!("CARGO_PKG_VERSION")
        ),
        InformationRequest::Version => println!(
            "sdkwork-intelligence-knowledgebase-rpc-bin {}",
            env!("CARGO_PKG_VERSION")
        ),
    }
}

fn install_process_crypto_provider() -> Result<(), std::io::Error> {
    if rustls::crypto::CryptoProvider::get_default().is_some() {
        return Ok(());
    }

    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .map_err(|_| std::io::Error::other("failed to install the process-level Rustls provider"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn installs_crypto_provider_before_rpc_bootstrap() {
        install_process_crypto_provider().expect("crypto provider should install");
        assert!(rustls::crypto::CryptoProvider::get_default().is_some());
    }

    #[test]
    fn recognizes_information_flags_before_runtime_configuration() {
        assert_eq!(
            information_request(["--help".to_owned()]),
            Some(InformationRequest::Help)
        );
        assert_eq!(
            information_request(["--version".to_owned()]),
            Some(InformationRequest::Version)
        );
        assert_eq!(information_request(["--serve".to_owned()]), None);
    }
}
