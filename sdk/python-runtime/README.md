# Firefam CLI Runtime for Python SDK

Platform-specific runtime package consumed by the published `openai-firefam`.

This package is staged during release so the SDK can pin an exact Firefam CLI
version without checking platform binaries into the repo.

`firefam-cli-bin` is intentionally wheel-only. Do not build or publish an
sdist for this package.
