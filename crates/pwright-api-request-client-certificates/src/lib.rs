//! Optional bounded PEM and PKCS#12 client-certificate decoding.
//!
//! File I/O and language-object conversion stay outside this crate. The output
//! is an origin-scoped Rustls identity consumed by the independent connector.

use openssl::{pkcs12::Pkcs12, pkey::PKey, x509::X509};
use pwright_api_request_connector_rustls::{
    ApiConnectionError, ApiConnectionErrorKind, RustlsClientIdentity, RustlsClientIdentityProvider,
};
use url::Url;

const MAX_IDENTITIES: usize = 64;
const MAX_MATERIAL_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Debug)]
pub enum ClientCertificateMaterial {
    Pem {
        certificate: Vec<u8>,
        private_key: Vec<u8>,
        passphrase: Option<String>,
    },
    Pfx {
        archive: Vec<u8>,
        passphrase: Option<String>,
    },
}

#[derive(Clone, Debug)]
pub struct ClientCertificateInput {
    pub origin: String,
    pub material: ClientCertificateMaterial,
}

#[derive(Clone, Debug)]
pub struct ClientCertificateStore {
    inputs: Vec<ClientCertificateInput>,
}

impl ClientCertificateStore {
    /// Retain bounded raw material without parsing it before a matching HTTPS request.
    ///
    /// # Errors
    ///
    /// Returns an error for excessive, empty or duplicate-origin material.
    pub fn new(mut inputs: Vec<ClientCertificateInput>) -> Result<Self, ClientCertificateError> {
        if inputs.len() > MAX_IDENTITIES {
            return Err(ClientCertificateError::new(format!(
                "client certificate count exceeds {MAX_IDENTITIES}"
            )));
        }
        for input in &mut inputs {
            input.origin = normalize_origin(&input.origin);
            match &input.material {
                ClientCertificateMaterial::Pem {
                    certificate,
                    private_key,
                    ..
                } => {
                    check_material_size("certificate", certificate)?;
                    check_material_size("private key", private_key)?;
                }
                ClientCertificateMaterial::Pfx { archive, .. } => {
                    check_material_size("PKCS#12 archive", archive)?;
                }
            }
        }
        for (index, input) in inputs.iter().enumerate() {
            if inputs[..index]
                .iter()
                .any(|earlier| earlier.origin == input.origin)
            {
                return Err(ClientCertificateError::new(format!(
                    "multiple client certificates for origin {} are not supported yet",
                    input.origin
                )));
            }
        }
        Ok(Self { inputs })
    }
}

impl RustlsClientIdentityProvider for ClientCertificateStore {
    fn identity_for_origin(
        &self,
        origin: &str,
    ) -> Result<Option<RustlsClientIdentity>, ApiConnectionError> {
        self.inputs
            .iter()
            .find(|input| input.origin == origin)
            .cloned()
            .map(load_client_identity)
            .transpose()
            .map_err(|error| {
                ApiConnectionError::new(ApiConnectionErrorKind::Tls, error.to_string())
            })
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{message}")]
pub struct ClientCertificateError {
    message: String,
}

impl ClientCertificateError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

/// Decode a bounded set of origin-scoped identities.
///
/// # Errors
///
/// Returns a stable error for capacity violations, malformed PEM/PKCS#12,
/// missing certificate/key material, wrong passphrases, or Rustls-incompatible
/// identities.
pub fn load_client_identities(
    inputs: Vec<ClientCertificateInput>,
) -> Result<Vec<RustlsClientIdentity>, ClientCertificateError> {
    ClientCertificateStore::new(inputs)?
        .inputs
        .into_iter()
        .map(load_client_identity)
        .collect()
}

fn load_client_identity(
    input: ClientCertificateInput,
) -> Result<RustlsClientIdentity, ClientCertificateError> {
    let (certificate_chain, private_key_pkcs8) = match input.material {
        ClientCertificateMaterial::Pem {
            certificate,
            private_key,
            passphrase,
        } => {
            check_material_size("certificate", &certificate)?;
            check_material_size("private key", &private_key)?;
            let certificates = X509::stack_from_pem(&certificate)
                .map_err(|error| certificate_error("failed to parse PEM certificate", &error))?;
            if certificates.is_empty() {
                return Err(ClientCertificateError::new(
                    "Failed to load client certificate: PEM certificate chain is empty",
                ));
            }
            let private_key = match passphrase {
                Some(passphrase) => {
                    PKey::private_key_from_pem_passphrase(&private_key, passphrase.as_bytes())
                }
                None => PKey::private_key_from_pem(&private_key),
            }
            .map_err(|error| certificate_error("failed to parse PEM private key", &error))?;
            let certificate_chain = certificates
                .into_iter()
                .map(|certificate| {
                    certificate
                        .to_der()
                        .map_err(|error| certificate_error("failed to encode certificate", &error))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let private_key_pkcs8 = private_key
                .private_key_to_pkcs8()
                .map_err(|error| certificate_error("failed to encode private key", &error))?;
            (certificate_chain, private_key_pkcs8)
        }
        ClientCertificateMaterial::Pfx {
            archive,
            passphrase,
        } => {
            check_material_size("PKCS#12 archive", &archive)?;
            let archive = Pkcs12::from_der(&archive)
                .map_err(|error| pfx_error("failed to parse PKCS#12 archive", &error))?;
            let parsed = archive
                .parse2(passphrase.as_deref().unwrap_or_default())
                .map_err(|error| pfx_error("failed to unlock PKCS#12 archive", &error))?;
            let certificate = parsed.cert.ok_or_else(|| {
                ClientCertificateError::new(
                    "Failed to load client certificate: PKCS#12 archive has no certificate",
                )
            })?;
            let private_key = parsed.pkey.ok_or_else(|| {
                ClientCertificateError::new(
                    "Failed to load client certificate: PKCS#12 archive has no private key",
                )
            })?;
            let mut certificate_chain = vec![
                certificate
                    .to_der()
                    .map_err(|error| certificate_error("failed to encode certificate", &error))?,
            ];
            if let Some(authorities) = parsed.ca {
                for certificate in authorities {
                    certificate_chain.push(certificate.to_der().map_err(|error| {
                        certificate_error("failed to encode certificate chain", &error)
                    })?);
                }
            }
            let private_key_pkcs8 = private_key
                .private_key_to_pkcs8()
                .map_err(|error| certificate_error("failed to encode private key", &error))?;
            (certificate_chain, private_key_pkcs8)
        }
    };
    RustlsClientIdentity::new(input.origin, certificate_chain, private_key_pkcs8)
        .map_err(|error| ClientCertificateError::new(error.message))
}

fn check_material_size(name: &str, material: &[u8]) -> Result<(), ClientCertificateError> {
    if material.is_empty() {
        return Err(ClientCertificateError::new(format!(
            "Failed to load client certificate: {name} is empty"
        )));
    }
    if material.len() > MAX_MATERIAL_BYTES {
        return Err(ClientCertificateError::new(format!(
            "Failed to load client certificate: {name} exceeds {MAX_MATERIAL_BYTES} bytes"
        )));
    }
    Ok(())
}

fn certificate_error(stage: &str, error: &impl std::fmt::Display) -> ClientCertificateError {
    ClientCertificateError::new(format!(
        "Failed to load client certificate: {stage}: {error}"
    ))
}

fn pfx_error(stage: &str, error: &impl std::fmt::Display) -> ClientCertificateError {
    let message = error.to_string();
    if message.contains("mac verify failure") {
        return ClientCertificateError::new(message);
    }
    ClientCertificateError::new(format!(
        "Failed to load client certificate: {stage}: Unsupported TLS certificate: {message}"
    ))
}

fn normalize_origin(origin: &str) -> String {
    Url::parse(origin).map_or_else(
        |_| origin.to_owned(),
        |url| url.origin().ascii_serialization(),
    )
}

#[cfg(test)]
mod tests {
    use openssl::{pkcs12::Pkcs12, pkey::PKey, x509::X509};
    use pwright_api_request_connector_rustls::RustlsClientIdentityProvider;
    use rcgen::{CertificateParams, KeyPair};

    use super::{
        ClientCertificateInput, ClientCertificateMaterial, ClientCertificateStore,
        load_client_identities,
    };

    fn pem_identity() -> (String, String) {
        let key = KeyPair::generate().expect("test key should generate");
        let certificate = CertificateParams::new(vec!["client.pwright.test".into()])
            .expect("certificate params should build")
            .self_signed(&key)
            .expect("certificate should self-sign");
        (certificate.pem(), key.serialize_pem())
    }

    #[test]
    fn pem_identity_decodes_without_file_or_network_io() {
        // Arrange
        let (certificate, private_key) = pem_identity();
        let input = ClientCertificateInput {
            origin: "https://example.test:443/path".into(),
            material: ClientCertificateMaterial::Pem {
                certificate: certificate.into_bytes(),
                private_key: private_key.into_bytes(),
                passphrase: None,
            },
        };

        // Act
        let identities = load_client_identities(vec![input]).expect("PEM identity should decode");

        // Assert
        assert_eq!(identities.len(), 1);
    }

    #[test]
    fn pfx_identity_decodes_with_the_exact_passphrase() {
        // Arrange
        let (certificate, private_key) = pem_identity();
        let certificate =
            X509::from_pem(certificate.as_bytes()).expect("test certificate should parse");
        let private_key =
            PKey::private_key_from_pem(private_key.as_bytes()).expect("test key should parse");
        let mut builder = Pkcs12::builder();
        builder
            .name("pwright-test")
            .pkey(&private_key)
            .cert(&certificate);
        let archive = builder
            .build2("secure")
            .expect("test archive should build")
            .to_der()
            .expect("test archive should encode");
        let input = ClientCertificateInput {
            origin: "https://example.test".into(),
            material: ClientCertificateMaterial::Pfx {
                archive,
                passphrase: Some("secure".into()),
            },
        };

        // Act
        let identities =
            load_client_identities(vec![input]).expect("PKCS#12 identity should decode");

        // Assert
        assert_eq!(identities.len(), 1);
    }

    #[test]
    fn wrong_pfx_passphrase_preserves_mac_verification_failure() {
        // Arrange
        let (certificate, private_key) = pem_identity();
        let certificate =
            X509::from_pem(certificate.as_bytes()).expect("test certificate should parse");
        let private_key =
            PKey::private_key_from_pem(private_key.as_bytes()).expect("test key should parse");
        let mut builder = Pkcs12::builder();
        builder
            .name("pwright-test")
            .pkey(&private_key)
            .cert(&certificate);
        let archive = builder
            .build2("secure")
            .expect("test archive should build")
            .to_der()
            .expect("test archive should encode");
        let input = ClientCertificateInput {
            origin: "https://example.test".into(),
            material: ClientCertificateMaterial::Pfx {
                archive,
                passphrase: Some("incorrect".into()),
            },
        };

        // Act
        let error = load_client_identities(vec![input]).expect_err("wrong passphrase should fail");

        // Assert
        assert!(error.to_string().contains("mac verify failure"));
    }

    #[test]
    fn store_defers_pfx_decoding_until_its_exact_origin_is_requested() {
        // Arrange
        let store = ClientCertificateStore::new(vec![ClientCertificateInput {
            origin: "https://example.test/path".into(),
            material: ClientCertificateMaterial::Pfx {
                archive: vec![1, 2, 3],
                passphrase: Some("secure".into()),
            },
        }])
        .expect("raw bounded material should be retained lazily");

        // Act
        let unrelated = store
            .identity_for_origin("https://unrelated.test")
            .expect("unrelated origin should not decode material");
        let matching = store.identity_for_origin("https://example.test");

        // Assert
        assert!(unrelated.is_none());
        assert!(matching.is_err());
    }
}
