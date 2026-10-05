use super::{ALPN, IO_TIMEOUT_MS, MAX_FRAME, Result};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use std::{
    net::{IpAddr, SocketAddr},
    path::Path,
    sync::Arc,
    time::Duration,
};

pub struct Credentials {
    pub certificate_chain: Vec<CertificateDer<'static>>,
    pub private_key: PrivateKeyDer<'static>,
    pub trust_roots: Vec<CertificateDer<'static>>,
}
impl Credentials {
    /// Explicit DER files; keys must be ordinary private files. No discovery,
    /// self-pairing, certificate issuance, secret logging or trust-on-first-use.
    pub fn load_der(certificate: &Path, private_key: &Path, ca: &Path) -> Result<Self> {
        let metadata = std::fs::symlink_metadata(private_key).map_err(|e| e.to_string())?;
        if !metadata.is_file() {
            return Err("private key must be an ordinary file".into());
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if metadata.permissions().mode() & 0o077 != 0 {
                return Err("private key permissions must exclude group and other".into());
            }
        }
        let read = |path: &Path| -> Result<Vec<u8>> {
            let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
            if !metadata.is_file() || metadata.len() > MAX_FRAME as u64 {
                return Err("credential file capacity/type".into());
            }
            std::fs::read(path).map_err(|e| e.to_string())
        };
        Ok(Self {
            certificate_chain: vec![CertificateDer::from(read(certificate)?)],
            private_key: PrivateKeyDer::try_from(read(private_key)?).map_err(str::to_owned)?,
            trust_roots: vec![CertificateDer::from(read(ca)?)],
        })
    }
    fn roots(&self) -> Result<Arc<rustls::RootCertStore>> {
        if self.certificate_chain.is_empty()
            || self.trust_roots.is_empty()
            || self.certificate_chain.len() > 8
            || self.trust_roots.len() > 64
        {
            return Err("credential chain capacity".into());
        }
        let mut roots = rustls::RootCertStore::empty();
        for cert in &self.trust_roots {
            roots.add(cert.clone()).map_err(|e| e.to_string())?;
        }
        Ok(Arc::new(roots))
    }
    pub fn server_config(&self) -> Result<quinn::ServerConfig> {
        let provider = Arc::new(rustls::crypto::ring::default_provider());
        let verifier = rustls::server::WebPkiClientVerifier::builder_with_provider(
            self.roots()?,
            provider.clone(),
        )
        .build()
        .map_err(|e| e.to_string())?;
        let mut crypto = rustls::ServerConfig::builder_with_provider(provider)
            .with_protocol_versions(&[&rustls::version::TLS13])
            .map_err(|e| e.to_string())?
            .with_client_cert_verifier(verifier)
            .with_single_cert(self.certificate_chain.clone(), self.private_key.clone_key())
            .map_err(|e| e.to_string())?;
        crypto.alpn_protocols = vec![ALPN.to_vec()];
        crypto.max_early_data_size = 0;
        crypto.send_tls13_tickets = 0;
        let crypto =
            quinn::crypto::rustls::QuicServerConfig::try_from(crypto).map_err(|e| e.to_string())?;
        let mut config = quinn::ServerConfig::with_crypto(Arc::new(crypto));
        config.transport_config(transport());
        config.migration(false);
        Ok(config)
    }
    pub fn client_config(&self) -> Result<quinn::ClientConfig> {
        let mut crypto = rustls::ClientConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_protocol_versions(&[&rustls::version::TLS13])
        .map_err(|e| e.to_string())?
        .with_root_certificates(self.roots()?)
        .with_client_auth_cert(self.certificate_chain.clone(), self.private_key.clone_key())
        .map_err(|e| e.to_string())?;
        crypto.alpn_protocols = vec![ALPN.to_vec()];
        crypto.enable_early_data = false;
        crypto.resumption = rustls::client::Resumption::disabled();
        let crypto =
            quinn::crypto::rustls::QuicClientConfig::try_from(crypto).map_err(|e| e.to_string())?;
        let mut config = quinn::ClientConfig::new(Arc::new(crypto));
        config.transport_config(transport());
        Ok(config)
    }
}
fn transport() -> Arc<quinn::TransportConfig> {
    let mut config = quinn::TransportConfig::default();
    config.max_concurrent_bidi_streams(1u8.into());
    config.max_concurrent_uni_streams(0u8.into());
    config.stream_receive_window((MAX_FRAME as u32 + 4).into());
    config.receive_window((MAX_FRAME as u32 + 4).into());
    config.send_window((MAX_FRAME + 4) as u64);
    config.datagram_receive_buffer_size(Some(256 * 1232));
    config.datagram_send_buffer_size(256 * 1232);
    config.max_idle_timeout(Some(quinn::VarInt::from_u32(IO_TIMEOUT_MS as u32).into()));
    config.keep_alive_interval(Some(Duration::from_millis(500)));
    // Conservative MTU. Media grouping uses the actual connection limit below it.
    config.initial_mtu(1200);
    config.mtu_discovery_config(None);
    Arc::new(config)
}
pub fn validate_private_address(address: SocketAddr) -> Result<()> {
    let allowed = match address.ip() {
        IpAddr::V4(ip) => ip.is_loopback() || ip.is_private(),
        IpAddr::V6(ip) => ip.is_loopback() || ip.is_unique_local(),
    };
    if !allowed {
        return Err("explicit private or loopback address required".into());
    }
    Ok(())
}
pub fn server_endpoint(address: SocketAddr, credentials: &Credentials) -> Result<quinn::Endpoint> {
    validate_private_address(address)?;
    quinn::Endpoint::server(credentials.server_config()?, address).map_err(|e| e.to_string())
}
pub fn client_endpoint(address: SocketAddr, credentials: &Credentials) -> Result<quinn::Endpoint> {
    validate_private_address(address)?;
    let mut endpoint = quinn::Endpoint::client(address).map_err(|e| e.to_string())?;
    endpoint.set_default_client_config(credentials.client_config()?);
    Ok(endpoint)
}
