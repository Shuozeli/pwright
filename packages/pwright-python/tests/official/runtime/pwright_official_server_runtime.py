"""Bind the unchanged upstream fixture servers to this Mac's Tailscale address."""

from __future__ import annotations

import atexit
import os
import subprocess
import sys
import tempfile
from collections.abc import Awaitable, Callable, Mapping
from importlib import import_module
from pathlib import Path
from typing import Any, cast
from urllib.parse import urlsplit, urlunsplit

_secure_certificate_directory: tempfile.TemporaryDirectory[str] | None = None


def _remove_secure_certificate() -> None:
    global _secure_certificate_directory
    directory = _secure_certificate_directory
    if directory is None:
        return
    directory.cleanup()
    _secure_certificate_directory = None


def _install_secure_certificate(tailscale_host: str) -> tuple[Path, Path]:
    global _secure_certificate_directory
    if _secure_certificate_directory is not None:
        raise RuntimeError("secure HTTP fixture certificate is already installed")
    directory = tempfile.TemporaryDirectory(prefix="pwright-official-tls-")
    certificate_path = Path(directory.name) / "certificate.pem"
    private_key_path = Path(directory.name) / "private-key.pem"
    try:
        subprocess.run(
            [
                "tailscale",
                "cert",
                "--cert-file",
                str(certificate_path),
                "--key-file",
                str(private_key_path),
                tailscale_host,
            ],
            check=True,
            capture_output=True,
            text=True,
        )
    except subprocess.CalledProcessError as error:
        directory.cleanup()
        detail = (error.stderr or error.stdout or "unknown tailscale cert error").strip()
        raise RuntimeError(
            f"failed to issue the secure HTTP fixture certificate: {detail}"
        ) from error
    certificate_path.chmod(0o600)
    private_key_path.chmod(0o600)
    _secure_certificate_directory = directory
    atexit.register(_remove_secure_certificate)
    return certificate_path, private_key_path


def _adapt_client_certificate_url(value: object, tailscale_host: str) -> object:
    if not isinstance(value, str):
        return value
    parsed = urlsplit(value)
    if parsed.scheme != "https" or parsed.port != 8000:
        return value
    if parsed.hostname == "localhost":
        host = f"{tailscale_host}."
    elif parsed.hostname == "127.0.0.1":
        host = tailscale_host
    else:
        return value
    return urlunsplit(parsed._replace(netloc=f"{host}:8000"))


def _adapt_client_certificates(value: object, tailscale_host: str) -> object:
    if not isinstance(value, list):
        return value
    adapted: list[object] = []
    for raw_certificate in cast(list[object], value):
        if not isinstance(raw_certificate, Mapping):
            adapted.append(raw_certificate)
            continue
        certificate = dict(cast(Mapping[object, object], raw_certificate))
        certificate["origin"] = _adapt_client_certificate_url(
            certificate.get("origin"), tailscale_host
        )
        adapted.append(certificate)
    return adapted


def _adapt_call_url(
    args: tuple[object, ...], kwargs: dict[str, object], tailscale_host: str
) -> tuple[tuple[object, ...], dict[str, object]]:
    adapted_args = list(args)
    adapted_kwargs = dict(kwargs)
    if adapted_args:
        adapted_args[0] = _adapt_client_certificate_url(adapted_args[0], tailscale_host)
    elif "url" in adapted_kwargs:
        adapted_kwargs["url"] = _adapt_client_certificate_url(adapted_kwargs["url"], tailscale_host)
    return tuple(adapted_args), adapted_kwargs


def pytest_configure() -> None:
    upstream_server = import_module("tests.server")

    tailscale_ip = os.environ["PWRIGHT_OFFICIAL_TAILSCALE_IP"]
    tailscale_host = os.environ["PWRIGHT_OFFICIAL_TAILSCALE_HOST"].removesuffix(".")

    secure_http_prefix: str | None = None
    secure_http_certificate: Any = None
    if os.environ.get("PWRIGHT_OFFICIAL_SECURE_HTTP_FIXTURE") == "1":
        certificate_path, private_key_path = _install_secure_certificate(tailscale_host)
        secure_http_certificate = upstream_server.ssl.PrivateCertificate.fromCertificateAndKeyPair(
            upstream_server.ssl.Certificate.loadPEM(certificate_path.read_bytes()),
            upstream_server.ssl.KeyPair.load(
                private_key_path.read_bytes(),
                upstream_server.crypto.FILETYPE_PEM,
            ),
        ).options()
        secure_http_prefix = f"https://{tailscale_host}:{upstream_server.test_server.server.PORT}"

    original_listen_ssl = upstream_server.reactor.listenSSL

    def listen_client_certificate_server(*args: object, **kwargs: object) -> object:
        if args and args[0] == 8000 and "interface" not in kwargs:
            kwargs["interface"] = tailscale_ip
        return original_listen_ssl(*args, **kwargs)

    upstream_server.reactor.listenSSL = listen_client_certificate_server

    def listen_http(server: Any, factory: Any) -> None:
        if secure_http_certificate is not None:
            server._listeners.append(
                upstream_server.reactor.listenSSL(
                    server.PORT,
                    factory,
                    secure_http_certificate,
                    interface=tailscale_ip,
                )
            )
            return
        server._listeners.append(
            upstream_server.reactor.listenTCP(
                server.PORT,
                factory,
                interface=tailscale_ip,
            )
        )

    def listen_https(server: Any, factory: Any) -> None:
        cert = upstream_server.ssl.PrivateCertificate.fromCertificateAndKeyPair(
            upstream_server.ssl.Certificate.loadPEM(
                (upstream_server._dirname / "testserver" / "cert.pem").read_bytes()
            ),
            upstream_server.ssl.KeyPair.load(
                (upstream_server._dirname / "testserver" / "key.pem").read_bytes(),
                upstream_server.crypto.FILETYPE_PEM,
            ),
        )
        server._listeners.append(
            upstream_server.reactor.listenSSL(
                server.PORT,
                factory,
                cert.options(),
                interface=tailscale_ip,
            )
        )

    upstream_server.HTTPServer.listen = listen_http
    upstream_server.HTTPSServer.listen = listen_https

    async_api = import_module("playwright.async_api")
    sync_api = import_module("playwright.sync_api")
    async_new_context = cast(Callable[..., Awaitable[object]], async_api.APIRequest.new_context)
    async_get = cast(Callable[..., Awaitable[object]], async_api.APIRequestContext.get)
    sync_new_context = cast(Callable[..., object], sync_api.APIRequest.new_context)
    sync_get = cast(Callable[..., object], sync_api.APIRequestContext.get)

    if secure_http_prefix is not None:
        print(
            f"[official-server] HTTP fixture uses trusted Tailscale TLS at {secure_http_prefix}",
            file=sys.stderr,
        )

    async def adapted_async_new_context(self: object, *args: object, **kwargs: object) -> object:
        adapted_kwargs = dict(kwargs)
        if "client_certificates" in adapted_kwargs:
            adapted_kwargs["client_certificates"] = _adapt_client_certificates(
                adapted_kwargs["client_certificates"], tailscale_host
            )
        return await async_new_context(self, *args, **adapted_kwargs)

    async def adapted_async_get(self: object, *args: object, **kwargs: object) -> object:
        adapted_args, adapted_kwargs = _adapt_call_url(args, kwargs, tailscale_host)
        return await async_get(self, *adapted_args, **adapted_kwargs)

    def adapted_sync_new_context(self: object, *args: object, **kwargs: object) -> object:
        adapted_kwargs = dict(kwargs)
        if "client_certificates" in adapted_kwargs:
            adapted_kwargs["client_certificates"] = _adapt_client_certificates(
                adapted_kwargs["client_certificates"], tailscale_host
            )
        return sync_new_context(self, *args, **adapted_kwargs)

    def adapted_sync_get(self: object, *args: object, **kwargs: object) -> object:
        adapted_args, adapted_kwargs = _adapt_call_url(args, kwargs, tailscale_host)
        return sync_get(self, *adapted_args, **adapted_kwargs)

    async_api.APIRequest.new_context = adapted_async_new_context
    async_api.APIRequestContext.get = adapted_async_get
    sync_api.APIRequest.new_context = adapted_sync_new_context
    sync_api.APIRequestContext.get = adapted_sync_get
    for server in (
        upstream_server.test_server.server,
        upstream_server.test_server.https_server,
    ):
        secure_http_server = (
            secure_http_prefix is not None and server is upstream_server.test_server.server
        )
        server.PREFIX = (
            secure_http_prefix
            if secure_http_server
            else f"{server.protocol}://{tailscale_host}:{server.PORT}"
        )
        server.EMPTY_PAGE = f"{server.PREFIX}/empty.html"
        cross_protocol = "https" if secure_http_server else server.protocol
        server.CROSS_PROCESS_PREFIX = f"{cross_protocol}://{tailscale_host}.:{server.PORT}"
    print(
        "[official-server] upstream fixture servers bind only to "
        f"{tailscale_ip} and use {tailscale_host}",
        file=sys.stderr,
    )


def pytest_unconfigure() -> None:
    _remove_secure_certificate()
