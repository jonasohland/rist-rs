pub mod basic;

/// DTLS transports
#[cfg(feature = "openssl")]
pub mod dtls {

    /// DTLS transport provided by OpenSSL
    #[cfg(feature = "openssl")]
    pub mod openssl {

        /// DTLS transport provided by OpenSSL
        pub use rist_rs_transport_dtls_openssl::Transport;
    }
}
