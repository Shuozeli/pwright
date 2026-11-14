//! Opt-in Node conversion boundary for pure persistent-Context decisions.

use napi_derive::napi;
use pwright_persistent_context_preflight::{
    Argument, BrowserEngine, CertificatePresence, DefaultArguments, verify_certificates,
    verify_default_arguments,
};

#[napi(object)]
pub struct NativeCertificatePresence {
    pub origin: bool,
    pub cert: bool,
    pub key: bool,
    pub pfx: bool,
    pub passphrase: bool,
}

/// Called after JavaScript has completed every certificate material read.
#[napi]
pub fn persistent_context_certificate_error(
    certificates: Vec<NativeCertificatePresence>,
) -> Option<String> {
    let presence: Vec<_> = certificates
        .into_iter()
        .map(|certificate| CertificatePresence {
            origin: certificate.origin,
            cert: certificate.cert,
            key: certificate.key,
            pfx: certificate.pfx,
            passphrase: certificate.passphrase,
        })
        .collect();
    verify_certificates(&presence)
        .err()
        .map(|error| error.message().to_owned())
}

/// Called only after certificate verification and boxed-string normalization.
#[napi]
pub fn persistent_context_argument_error(
    browser_name: String,
    args: Vec<Option<String>>,
    channel: Option<String>,
) -> napi::Result<Option<String>> {
    let engine = match browser_name.as_str() {
        "chromium" => BrowserEngine::Chromium,
        "firefox" => BrowserEngine::Firefox,
        "webkit" => BrowserEngine::WebKit,
        _ => {
            return Err(napi::Error::new(
                napi::Status::InvalidArg,
                "unknown browser engine",
            ));
        }
    };
    let arguments: Vec<_> = args
        .iter()
        .map(|argument| match argument.as_deref() {
            Some(value) => Argument::String(value),
            None => Argument::Other,
        })
        .collect();
    Ok(verify_default_arguments(
        engine,
        DefaultArguments {
            args: Some(&arguments),
            channel: channel.as_deref(),
            ignore_default_args_truthy_non_array: false,
        },
    )
    .err()
    .map(|error| error.message().to_owned()))
}
