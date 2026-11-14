#![cfg(feature = "live-tokio")]

use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use native_tls::{Certificate, Identity, TlsAcceptor, TlsConnector};
use openssl::{pkcs12::Pkcs12, pkey::PKey, x509::X509};
use rcgen::{BasicConstraints, CertificateParams, IsCa, KeyPair};
use tokio_native_tls::TlsAcceptor as TokioTlsAcceptor;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::protocol::Message;
use tokio_tungstenite::{Connector, client_async_tls_with_config};

const HOST: &str = "pwright-test.invalid";

fn test_identity() -> (Identity, Certificate) {
    let ca_key = KeyPair::generate().expect("CA key");
    let mut ca_params = CertificateParams::new(Vec::<String>::new()).expect("CA parameters");
    ca_params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    let ca_cert = ca_params.self_signed(&ca_key).expect("CA certificate");

    let server_key = KeyPair::generate().expect("server key");
    let server_params = CertificateParams::new(vec![HOST.to_owned()]).expect("server parameters");
    let server_cert = server_params
        .signed_by(&server_key, &ca_cert, &ca_key)
        .expect("server certificate");
    // macOS Security Framework cannot import rcgen's generated PKCS#8 key
    // directly, so package the same certificate and key as PKCS#12 in memory.
    let key = PKey::private_key_from_pem(server_key.serialize_pem().as_bytes()).expect("PEM key");
    let cert = X509::from_pem(server_cert.pem().as_bytes()).expect("PEM certificate");
    let mut archive = Pkcs12::builder();
    archive.name("pwright TLS test").pkey(&key).cert(&cert);
    let archive = archive.build2("test-only").expect("PKCS#12 archive");
    let identity = Identity::from_pkcs12(&archive.to_der().expect("PKCS#12 bytes"), "test-only")
        .expect("native TLS server identity");
    let root = Certificate::from_pem(ca_cert.pem().as_bytes()).expect("native TLS CA certificate");
    (identity, root)
}

async fn connect_over_duplex(
    request_host: &str,
    identity: Identity,
    connector: TlsConnector,
) -> Result<String, String> {
    let (client_io, server_io) = tokio::io::duplex(32 * 1024);
    let acceptor = TokioTlsAcceptor::from(
        TlsAcceptor::builder(identity)
            .build()
            .expect("TLS acceptor"),
    );
    let server = tokio::spawn(async move {
        if let Ok(tls) = acceptor.accept(server_io).await
            && let Ok(mut ws) = tokio_tungstenite::accept_async(tls).await
        {
            ws.send(Message::Text("verified".into()))
                .await
                .expect("server message");
        }
    });

    let request = format!("wss://{request_host}/ws")
        .into_client_request()
        .expect("WSS request");
    let result = tokio::time::timeout(
        Duration::from_secs(5),
        client_async_tls_with_config(
            request,
            client_io,
            None,
            Some(Connector::NativeTls(connector)),
        ),
    )
    .await
    .expect("TLS connection completed within deadline");
    let result = match result {
        Ok((mut ws, _)) => match ws.next().await {
            Some(Ok(Message::Text(text))) => Ok(text.to_string()),
            other => Err(format!("unexpected server message: {other:?}")),
        },
        Err(error) => Err(error.to_string()),
    };
    tokio::time::timeout(Duration::from_secs(5), server)
        .await
        .expect("TLS server completed within deadline")
        .expect("TLS server task");
    result
}

#[tokio::test]
async fn trusted_ca_and_matching_hostname_are_accepted() {
    // Arrange
    let (identity, root) = test_identity();
    let mut builder = TlsConnector::builder();
    builder.add_root_certificate(root);
    let connector = builder.build().expect("trusted TLS connector");

    // Act
    let result = connect_over_duplex(HOST, identity, connector).await;

    // Assert
    assert_eq!(result, Ok("verified".to_owned()));
}

#[tokio::test]
async fn untrusted_issuer_is_rejected() {
    // Arrange
    let (identity, _) = test_identity();
    let connector = TlsConnector::builder()
        .build()
        .expect("default TLS connector");

    // Act
    let result = connect_over_duplex(HOST, identity, connector).await;

    // Assert
    assert!(result.is_err(), "untrusted issuer unexpectedly accepted");
}

#[tokio::test]
async fn hostname_mismatch_is_rejected_with_trusted_ca() {
    // Arrange
    let (identity, root) = test_identity();
    let mut builder = TlsConnector::builder();
    builder.add_root_certificate(root);
    let connector = builder.build().expect("trusted TLS connector");

    // Act
    let result = connect_over_duplex("other-pwright-test.invalid", identity, connector).await;

    // Assert
    assert!(result.is_err(), "hostname mismatch unexpectedly accepted");
}
