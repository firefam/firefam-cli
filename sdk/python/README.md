# Firefam Python SDK (Experimental)

Experimental Python SDK for `firefam app-server` JSON-RPC v2 over stdio, with a small default surface optimized for real scripts and apps.

The generated wire-model layer is sourced from the pinned `firefam-cli-bin`
runtime package and exposed as Pydantic models with snake_case Python fields
that serialize back to the app-server’s camelCase wire format.
The package root exports the ergonomic client API; public app-server value and
event types live in `firefamai_firefam.types`.

## Install

```bash
cd sdk/python
uv sync
source .venv/bin/activate
```

Published SDK builds pin an exact `firefam-cli-bin` runtime dependency
with the same version as the SDK. Pass `AppServerConfig(firefam_bin=...)` only
when you intentionally want to run against a specific local app-server binary.

## Quickstart

```python
from firefamai_firefam import Firefam

with Firefam() as firefam:
    # Call login_api_key(...) first when this app-server session is not
    # already authenticated.
    thread = firefam.thread_start(model="gpt-5")
    result = thread.run("Say hello in one sentence.")
    print(result.final_response)
    print(len(result.items))
```

`thread.run(...)` and `thread.turn(...).run()` return `TurnResult`. Its
`final_response` is `None` when the turn completes without a final-answer or
phase-less assistant message item.

## Login

Use the auth helper that matches your app:

```python
from firefamai_firefam import Firefam

with Firefam() as firefam:
    firefam.login_api_key("sk-...")
    account = firefam.account()
    print(account.account)
```

Interactive ChatGPT login returns a handle. Open the provided URL or device-code
page, then wait for the matching completion event:

```python
with Firefam() as firefam:
    login = firefam.login_chatgpt()
    print(login.auth_url)
    completed = login.wait()
    print(completed.success)
```

Use `login_chatgpt_device_code()` for device-code auth, `handle.cancel()` to
stop an in-progress interactive login, and `logout()` to clear the active
app-server account session.

## Docs map

- Golden path tutorial: `docs/getting-started.md`
- API reference (signatures + behavior): `docs/api-reference.md`
- Common decisions and pitfalls: `docs/faq.md`
- Runnable examples index: `examples/README.md`
- Jupyter walkthrough notebook: `notebooks/sdk_walkthrough.ipynb`

## Examples

Start here:

```bash
cd sdk/python
python examples/01_quickstart_constructor/sync.py
python examples/01_quickstart_constructor/async.py
```

## Runtime

Published SDK builds are pinned to an exact `firefam-cli-bin` package
version, and that runtime package carries the platform-specific binary for the
target wheel. The SDK package version and runtime package version must match.

## Compatibility and versioning

- Package: `firefamai-firefam`
- Runtime package: `firefam-cli-bin`
- Python: `>=3.10`
- Target protocol: Firefam `app-server` JSON-RPC v2
- Versioning rule: the SDK package version is the underlying Firefam runtime version

## Notes

- `Firefam()` is eager and performs startup + `initialize` in the constructor.
- Use context managers (`with Firefam() as firefam:`) to ensure shutdown.
- Plain strings are accepted anywhere a turn input is accepted; they are
  shorthand for `TextInput(...)`.
- Prefer `thread.run("...")` for the common case. Use `thread.turn(...)` when
  you need streaming, steering, or interrupt control.
- For transient overload, use `retry_on_overload` from the package root.
