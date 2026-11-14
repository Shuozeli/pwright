//! Pure, opt-in decisions for `browserType.launchPersistentContext` preflight.
//!
//! The caller must first finish all client-side certificate material reads,
//! unwrap protocol boxed strings, and establish JavaScript truthiness. This
//! crate does no I/O and never owns a browser, process, transport, or runtime.
//! It preserves the order of the current Node preflight: certificates first,
//! then profile arguments, classic Firefox's juggler argument, Chromium's
//! pipe argument, and page arguments.

/// The browser type supplied by the public Playwright-style API.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BrowserEngine {
    Chromium,
    Firefox,
    WebKit,
}

/// Whether each value is JavaScript-truthy after client-side material reads.
///
/// In particular, a zero-length `Buffer` is truthy; a missing or empty string
/// is false. This type deliberately does not infer truthiness from Rust bytes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CertificatePresence {
    pub origin: bool,
    pub cert: bool,
    pub key: bool,
    pub pfx: bool,
    pub passphrase: bool,
}

/// An argument after the caller has unwrapped protocol boxed `String` values.
/// Non-string values remain inert here and belong to protocol validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Argument<'a> {
    String(&'a str),
    Other,
}

/// The already-normalized, browser-independent part of launch options.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DefaultArguments<'a> {
    /// `None` means `args` was not an Array; no argument check takes place.
    pub args: Option<&'a [Argument<'a>]>,
    /// `None` means `channel` was not a string after protocol normalization.
    pub channel: Option<&'a str>,
    /// `ignoreDefaultArgs` was truthy but not an Array. An Array, even an
    /// empty one, must be represented by `false` and is still checked.
    pub ignore_default_args_truthy_non_array: bool,
}

/// One complete, already-normalized preflight request.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PersistentContextPreflight<'a> {
    pub engine: BrowserEngine,
    pub certificates: &'a [CertificatePresence],
    pub arguments: DefaultArguments<'a>,
}

/// Exact public preflight reason, without its JavaScript API prefix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PreflightError {
    MissingCertificateOrigin,
    MissingCertificateMaterial,
    CertificateWithoutKey,
    KeyWithoutCertificate,
    PfxWithCertificateOrKey,
    ProfileArgument,
    FirefoxProfileArgument,
    FirefoxJugglerArgument,
    ChromiumRemoteDebuggingPipe,
    PageArgument,
}

impl PreflightError {
    pub const fn message(self) -> &'static str {
        match self {
            Self::MissingCertificateOrigin => "clientCertificates.origin is required",
            Self::MissingCertificateMaterial => "None of cert, key, passphrase or pfx is specified",
            Self::CertificateWithoutKey => "cert is specified without key",
            Self::KeyWithoutCertificate => "key is specified without cert",
            Self::PfxWithCertificateOrKey => {
                "pfx is specified together with cert, key or passphrase"
            }
            Self::ProfileArgument => {
                "Pass userDataDir parameter to 'browserType.launchPersistentContext(userDataDir, options)' instead of specifying '--user-data-dir' argument"
            }
            Self::FirefoxProfileArgument => {
                "Pass userDataDir parameter to 'browserType.launchPersistentContext(userDataDir, options)' instead of specifying '--profile' argument"
            }
            Self::FirefoxJugglerArgument => "Use the port parameter instead of -juggler argument",
            Self::ChromiumRemoteDebuggingPipe => {
                "Playwright manages remote debugging connection itself."
            }
            Self::PageArgument => "Arguments can not specify page to be opened",
        }
    }
}

impl std::fmt::Display for PreflightError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.message())
    }
}

impl std::error::Error for PreflightError {}

/// Verify certificate combinations in source order after all material reads.
pub fn verify_certificates(certificates: &[CertificatePresence]) -> Result<(), PreflightError> {
    for certificate in certificates {
        if !certificate.origin {
            return Err(PreflightError::MissingCertificateOrigin);
        }
        if !certificate.cert && !certificate.key && !certificate.passphrase && !certificate.pfx {
            return Err(PreflightError::MissingCertificateMaterial);
        }
        if certificate.cert && !certificate.key {
            return Err(PreflightError::CertificateWithoutKey);
        }
        if !certificate.cert && certificate.key {
            return Err(PreflightError::KeyWithoutCertificate);
        }
        if certificate.pfx && (certificate.cert || certificate.key) {
            return Err(PreflightError::PfxWithCertificateOrKey);
        }
    }
    Ok(())
}

/// Explicit alias for callers using Playwright's client-certificate naming.
pub use verify_certificates as verify_client_certificates;

/// Verify the selected Rust-backed Node profile's default-argument preflight.
pub fn verify_default_arguments(
    engine: BrowserEngine,
    options: DefaultArguments<'_>,
) -> Result<(), PreflightError> {
    let Some(args) = options.args else {
        return Ok(());
    };
    if options.ignore_default_args_truthy_non_array {
        return Ok(());
    }

    let profile_argument = args.iter().any(|arg| match (engine, arg) {
        (BrowserEngine::Firefox, Argument::String(value)) => {
            value.starts_with("-profile") || value.starts_with("--profile")
        }
        (_, Argument::String(value)) => value.starts_with("--user-data-dir"),
        (_, Argument::Other) => false,
    });
    if profile_argument {
        return Err(match engine {
            BrowserEngine::Firefox => PreflightError::FirefoxProfileArgument,
            BrowserEngine::Chromium | BrowserEngine::WebKit => PreflightError::ProfileArgument,
        });
    }

    let classic_firefox = engine == BrowserEngine::Firefox
        && !options
            .channel
            .is_some_and(|channel| channel.starts_with("moz-"));
    if classic_firefox
        && args
            .iter()
            .any(|arg| matches!(arg, Argument::String(value) if value.starts_with("-juggler")))
    {
        return Err(PreflightError::FirefoxJugglerArgument);
    }

    if engine == BrowserEngine::Chromium
        && args.iter().any(|arg| {
            matches!(arg, Argument::String(value) if value.starts_with("--remote-debugging-pipe"))
        })
    {
        return Err(PreflightError::ChromiumRemoteDebuggingPipe);
    }

    let rejects_page_argument = engine != BrowserEngine::Firefox
        || options
            .channel
            .is_some_and(|channel| channel.starts_with("moz-"));
    if rejects_page_argument
        && args
            .iter()
            .any(|arg| matches!(arg, Argument::String(value) if !value.starts_with('-')))
    {
        return Err(PreflightError::PageArgument);
    }

    Ok(())
}

/// Preserve the client certificate versus launch argument error order.
pub fn verify_persistent_context(
    input: PersistentContextPreflight<'_>,
) -> Result<(), PreflightError> {
    verify_certificates(input.certificates)?;
    verify_default_arguments(input.engine, input.arguments)
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALID_CERTIFICATE: CertificatePresence = CertificatePresence {
        origin: true,
        cert: true,
        key: true,
        pfx: false,
        passphrase: false,
    };

    #[test]
    fn empty_certificate_collection_is_valid() {
        // Arrange
        let certificates = [];

        // Act
        let result = verify_client_certificates(&certificates);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn certificate_combinations_match_all_truthiness_states() {
        // One independent Arrange/Act/Assert case for each input combination.
        for mask in 0u8..32 {
            // Arrange
            let certificate = CertificatePresence {
                origin: mask & 1 != 0,
                cert: mask & 2 != 0,
                key: mask & 4 != 0,
                pfx: mask & 8 != 0,
                passphrase: mask & 16 != 0,
            };
            let expected = if !certificate.origin {
                Err(PreflightError::MissingCertificateOrigin)
            } else if !certificate.cert
                && !certificate.key
                && !certificate.passphrase
                && !certificate.pfx
            {
                Err(PreflightError::MissingCertificateMaterial)
            } else if certificate.cert && !certificate.key {
                Err(PreflightError::CertificateWithoutKey)
            } else if !certificate.cert && certificate.key {
                Err(PreflightError::KeyWithoutCertificate)
            } else if certificate.pfx && (certificate.cert || certificate.key) {
                Err(PreflightError::PfxWithCertificateOrKey)
            } else {
                Ok(())
            };

            // Act
            let result = verify_client_certificates(&[certificate]);

            // Assert
            assert_eq!(result, expected, "combination {mask:06b}");
        }
    }

    #[test]
    fn certificate_errors_follow_input_order_not_error_severity() {
        // Arrange
        let certificates = [
            CertificatePresence {
                cert: true,
                key: false,
                ..VALID_CERTIFICATE
            },
            CertificatePresence::default(),
        ];

        // Act
        let result = verify_client_certificates(&certificates);

        // Assert
        assert_eq!(result, Err(PreflightError::CertificateWithoutKey));
    }

    #[test]
    fn pfx_and_passphrase_without_pem_are_valid() {
        // Arrange
        let certificates = [CertificatePresence {
            origin: true,
            pfx: true,
            passphrase: true,
            ..CertificatePresence::default()
        }];

        // Act
        let result = verify_client_certificates(&certificates);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn pfx_with_cert_and_key_rejects_after_pair_validation() {
        // Arrange
        let certificates = [CertificatePresence {
            pfx: true,
            ..VALID_CERTIFICATE
        }];

        // Act
        let result = verify_client_certificates(&certificates);

        // Assert
        assert_eq!(result, Err(PreflightError::PfxWithCertificateOrKey));
    }

    #[test]
    fn absent_argument_array_bypasses_argument_checks() {
        // Arrange
        let options = DefaultArguments {
            args: None,
            channel: Some("moz-firefox"),
            ignore_default_args_truthy_non_array: false,
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Firefox, options);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn truthy_non_array_ignore_default_args_bypasses_checks() {
        // Arrange
        let args = [
            Argument::String("--user-data-dir=x"),
            Argument::String("a-page"),
        ];
        let options = DefaultArguments {
            args: Some(&args),
            ignore_default_args_truthy_non_array: true,
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Chromium, options);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn array_ignore_default_args_does_not_bypass_checks() {
        // Arrange
        let args = [Argument::String("page")];
        let options = DefaultArguments {
            args: Some(&args),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Chromium, options);

        // Assert
        assert_eq!(result, Err(PreflightError::PageArgument));
    }

    #[test]
    fn chromium_profile_argument_precedes_remote_pipe_and_page() {
        // Arrange
        let args = [
            Argument::String("page"),
            Argument::String("--remote-debugging-pipe=x"),
            Argument::String("--user-data-dir=x"),
        ];
        let options = DefaultArguments {
            args: Some(&args),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Chromium, options);

        // Assert
        assert_eq!(result, Err(PreflightError::ProfileArgument));
    }

    #[test]
    fn chromium_remote_pipe_precedes_page() {
        // Arrange
        let args = [
            Argument::String("page"),
            Argument::String("--remote-debugging-pipe"),
        ];
        let options = DefaultArguments {
            args: Some(&args),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Chromium, options);

        // Assert
        assert_eq!(result, Err(PreflightError::ChromiumRemoteDebuggingPipe));
    }

    #[test]
    fn firefox_profile_prefixes_are_rejected() {
        // One independent Arrange/Act/Assert case for each accepted prefix.
        for profile in [
            "-profile",
            "-profile-extra",
            "--profile=path",
            "--profile-extra",
        ] {
            // Arrange
            let args = [Argument::String(profile)];
            let options = DefaultArguments {
                args: Some(&args),
                ..DefaultArguments::default()
            };

            // Act
            let result = verify_default_arguments(BrowserEngine::Firefox, options);

            // Assert
            assert_eq!(
                result,
                Err(PreflightError::FirefoxProfileArgument),
                "{profile}"
            );
        }
    }

    #[test]
    fn classic_firefox_rejects_every_juggler_prefix() {
        // One independent Arrange/Act/Assert case for each upstream prefix.
        for juggler in [
            "-juggler",
            "-juggler-pipe",
            "-juggler-port=0",
            "-jugglerExtra",
        ] {
            // Arrange
            let args = [Argument::String(juggler)];
            let options = DefaultArguments {
                args: Some(&args),
                ..DefaultArguments::default()
            };

            // Act
            let result = verify_default_arguments(BrowserEngine::Firefox, options);

            // Assert
            assert_eq!(
                result,
                Err(PreflightError::FirefoxJugglerArgument),
                "{juggler}"
            );
        }
    }

    #[test]
    fn classic_firefox_profile_argument_precedes_juggler() {
        // Arrange
        let args = [
            Argument::String("-juggler-pipe"),
            Argument::String("--profile=unused"),
        ];
        let options = DefaultArguments {
            args: Some(&args),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Firefox, options);

        // Assert
        assert_eq!(result, Err(PreflightError::FirefoxProfileArgument));
    }

    #[test]
    fn bidi_firefox_does_not_reject_juggler_prefix() {
        // Arrange
        let args = [Argument::String("-juggler-pipe")];
        let options = DefaultArguments {
            args: Some(&args),
            channel: Some("moz-firefox"),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Firefox, options);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn classic_firefox_juggler_match_is_single_dash_and_case_sensitive() {
        // One independent Arrange/Act/Assert case for each non-matching input.
        for argument in ["--juggler-pipe", "-Juggler-pipe", "-other-juggler"] {
            // Arrange
            let args = [Argument::String(argument)];
            let options = DefaultArguments {
                args: Some(&args),
                ..DefaultArguments::default()
            };

            // Act
            let result = verify_default_arguments(BrowserEngine::Firefox, options);

            // Assert
            assert_eq!(result, Ok(()), "{argument}");
        }
    }

    #[test]
    fn bidi_firefox_page_error_is_not_replaced_by_juggler_error() {
        // Arrange
        let args = [
            Argument::String("-juggler-pipe"),
            Argument::String("https://example.invalid/"),
        ];
        let options = DefaultArguments {
            args: Some(&args),
            channel: Some("moz-firefox"),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Firefox, options);

        // Assert
        assert_eq!(result, Err(PreflightError::PageArgument));
    }

    #[test]
    fn ignored_default_arguments_do_not_reject_juggler() {
        // Arrange
        let args = [Argument::String("-juggler-pipe")];
        let options = DefaultArguments {
            args: Some(&args),
            ignore_default_args_truthy_non_array: true,
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Firefox, options);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn certificate_error_precedes_classic_firefox_juggler() {
        // Arrange
        let args = [Argument::String("-juggler-pipe")];
        let input = PersistentContextPreflight {
            engine: BrowserEngine::Firefox,
            certificates: &[CertificatePresence::default()],
            arguments: DefaultArguments {
                args: Some(&args),
                ..DefaultArguments::default()
            },
        };

        // Act
        let result = verify_persistent_context(input);

        // Assert
        assert_eq!(result, Err(PreflightError::MissingCertificateOrigin));
    }

    #[test]
    fn webkit_profile_argument_precedes_page() {
        // Arrange
        let args = [
            Argument::String("page"),
            Argument::String("--user-data-dir-extra"),
        ];
        let options = DefaultArguments {
            args: Some(&args),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::WebKit, options);

        // Assert
        assert_eq!(result, Err(PreflightError::ProfileArgument));
    }

    #[test]
    fn firefox_classic_channel_allows_page_argument() {
        // Arrange
        let args = [Argument::String("https://example.invalid/")];
        let options = DefaultArguments {
            args: Some(&args),
            channel: Some("firefox"),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Firefox, options);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn firefox_moz_channel_rejects_page_argument() {
        // Arrange
        let args = [Argument::String("https://example.invalid/")];
        let options = DefaultArguments {
            args: Some(&args),
            channel: Some("moz-firefox"),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Firefox, options);

        // Assert
        assert_eq!(result, Err(PreflightError::PageArgument));
    }

    #[test]
    fn firefox_channel_prefix_is_case_sensitive() {
        // Arrange
        let args = [Argument::String("page")];
        let options = DefaultArguments {
            args: Some(&args),
            channel: Some("Moz-firefox"),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Firefox, options);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn non_string_arguments_are_inert() {
        // Arrange
        let args = [Argument::Other];
        let options = DefaultArguments {
            args: Some(&args),
            channel: Some("moz-firefox"),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::Firefox, options);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn chromium_remote_pipe_is_not_rejected_for_webkit() {
        // Arrange
        let args = [Argument::String("--remote-debugging-pipe")];
        let options = DefaultArguments {
            args: Some(&args),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::WebKit, options);

        // Assert
        assert_eq!(result, Ok(()));
    }

    #[test]
    fn empty_string_is_a_page_argument_for_non_firefox_engines() {
        // Arrange
        let args = [Argument::String("")];
        let options = DefaultArguments {
            args: Some(&args),
            ..DefaultArguments::default()
        };

        // Act
        let result = verify_default_arguments(BrowserEngine::WebKit, options);

        // Assert
        assert_eq!(result, Err(PreflightError::PageArgument));
    }

    #[test]
    fn certificate_error_precedes_argument_error() {
        // Arrange
        let certificates = [CertificatePresence::default()];
        let args = [Argument::String("--user-data-dir=x")];
        let input = PersistentContextPreflight {
            engine: BrowserEngine::Chromium,
            certificates: &certificates,
            arguments: DefaultArguments {
                args: Some(&args),
                ..DefaultArguments::default()
            },
        };

        // Act
        let result = verify_persistent_context(input);

        // Assert
        assert_eq!(result, Err(PreflightError::MissingCertificateOrigin));
    }

    #[test]
    fn valid_certificate_continues_to_argument_validation() {
        // Arrange
        let args = [Argument::String("page")];
        let input = PersistentContextPreflight {
            engine: BrowserEngine::Chromium,
            certificates: &[VALID_CERTIFICATE],
            arguments: DefaultArguments {
                args: Some(&args),
                ..DefaultArguments::default()
            },
        };

        // Act
        let result = verify_persistent_context(input);

        // Assert
        assert_eq!(result, Err(PreflightError::PageArgument));
    }

    #[test]
    fn every_error_has_exact_public_reason_without_js_api_prefix() {
        // One independent Arrange/Act/Assert case for every error variant.
        let cases = [
            (
                PreflightError::MissingCertificateOrigin,
                "clientCertificates.origin is required",
            ),
            (
                PreflightError::MissingCertificateMaterial,
                "None of cert, key, passphrase or pfx is specified",
            ),
            (
                PreflightError::CertificateWithoutKey,
                "cert is specified without key",
            ),
            (
                PreflightError::KeyWithoutCertificate,
                "key is specified without cert",
            ),
            (
                PreflightError::PfxWithCertificateOrKey,
                "pfx is specified together with cert, key or passphrase",
            ),
            (
                PreflightError::ProfileArgument,
                "Pass userDataDir parameter to 'browserType.launchPersistentContext(userDataDir, options)' instead of specifying '--user-data-dir' argument",
            ),
            (
                PreflightError::FirefoxProfileArgument,
                "Pass userDataDir parameter to 'browserType.launchPersistentContext(userDataDir, options)' instead of specifying '--profile' argument",
            ),
            (
                PreflightError::FirefoxJugglerArgument,
                "Use the port parameter instead of -juggler argument",
            ),
            (
                PreflightError::ChromiumRemoteDebuggingPipe,
                "Playwright manages remote debugging connection itself.",
            ),
            (
                PreflightError::PageArgument,
                "Arguments can not specify page to be opened",
            ),
        ];
        for (error, expected) in cases {
            // Arrange: use this case's error and exact expected message.

            // Act
            let message = error.message();

            // Assert
            assert_eq!(message, expected);
            assert_eq!(error.to_string(), expected);
        }
    }
}
