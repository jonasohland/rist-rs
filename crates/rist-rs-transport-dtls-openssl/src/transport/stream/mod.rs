pub mod non_blocking;

use std::fmt::Debug;

use openssl::error::ErrorStack as SslError;
use openssl::pkey;
use openssl::ssl;
use openssl::x509;

/// Trait that can be implemented to provide an SslContext for
pub trait SslContextProvider: Send + Sync + Clone + Debug {
    type Error;
    fn context(&mut self) -> Result<&ssl::SslContext, Self::Error>;
}

/// Simple provider that wraps a single SslContext
#[derive(Debug, Clone)]
pub struct SimpleContextProvider(ssl::SslContext);

/// Builder for SimpleContextProvider
pub struct SimpleContextProviderBuilder {
    builder: ssl::SslAcceptorBuilder,
}

impl SimpleContextProviderBuilder {
    /// Set the certificate for the peer
    pub fn with_certificate(mut self, cert: &x509::X509Ref) -> Result<Self, SslError> {
        self.builder.set_certificate(cert.as_ref())?;
        Ok(self)
    }

    /// Add the private key for the peer
    pub fn with_key<T>(mut self, key: &pkey::PKeyRef<T>) -> Result<Self, SslError>
    where
        T: pkey::HasPrivate,
    {
        self.builder.set_private_key::<T>(key)?;
        Ok(self)
    }

    /// Set the name that should be verified for the remote peer
    pub fn with_expected_peer_name(mut self, name: &str) -> Result<Self, SslError> {
        self.builder.verify_param_mut().set_host(name)?;
        Ok(self)
    }

    /// Additionally verify the client certificate
    pub fn with_verify_client_cert(mut self) -> Self {
        self.builder
            .set_verify(ssl::SslVerifyMode::PEER | ssl::SslVerifyMode::FAIL_IF_NO_PEER_CERT);
        self
    }

    /// Set a single trusted CA certificate for peer certificate verification
    pub fn with_ca_cert(mut self, cert: &x509::X509Ref) -> Result<Self, SslError> {
        let mut store = x509::store::X509StoreBuilder::new()?;
        store.add_cert(cert.to_owned())?;
        self.builder.set_cert_store(store.build());
        Ok(self)
    }

    /// Disable certificate verification
    pub fn with_no_verify(mut self) -> Self {
        #[cfg(not(feature = "ssl_no_verify_dont_warn"))]
        tracing::warn!("certificate verification disabled");
        self.builder.set_verify(ssl::SslVerifyMode::NONE);
        self
    }

    /// Build the provider
    pub fn build(self) -> SimpleContextProvider {
        SimpleContextProvider::new(self.builder.build().into_context())
    }
}

impl SimpleContextProvider {
    /// Create a new provider by wrapping a context
    pub fn new(ctx: ssl::SslContext) -> Self {
        Self(ctx)
    }

    /// Create a builder
    pub fn builder() -> Result<SimpleContextProviderBuilder, SslError> {
        Ok(SimpleContextProviderBuilder {
            builder: ssl::SslAcceptor::mozilla_intermediate_v5(ssl::SslMethod::dtls())?,
        })
    }
}

impl SslContextProvider for SimpleContextProvider {
    type Error = openssl::error::ErrorStack;

    fn context(&mut self) -> Result<&ssl::SslContext, Self::Error> {
        Ok(&self.0)
    }
}

/// DTLS protected stream configuration
#[derive(Debug, Clone)]
pub struct Config<C>
where
    C: SslContextProvider,
{
    context_builder: C,
}

impl<C> Config<C>
where
    C: SslContextProvider,
{
    /// Create a new stream configuration
    pub fn new(context_builder: C) -> Self {
        Self { context_builder }
    }
}