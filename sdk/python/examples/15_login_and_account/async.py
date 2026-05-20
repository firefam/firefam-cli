import sys
from pathlib import Path

_EXAMPLES_ROOT = Path(__file__).resolve().parents[1]
if str(_EXAMPLES_ROOT) not in sys.path:
    sys.path.insert(0, str(_EXAMPLES_ROOT))

from _bootstrap import ensure_local_sdk_src, runtime_config

ensure_local_sdk_src()

import asyncio

from firefamai_firefam import AsyncFirefam


async def main() -> None:
    async with AsyncFirefam(config=runtime_config()) as firefam:
        # Browser login returns a live handle. Open `auth_url` and await `wait()`
        # in a real app; this example cancels immediately so it stays non-blocking.
        login = await firefam.login_chatgpt()
        canceled = await login.cancel()
        completed = await login.wait()
        account = await firefam.account()

        print("login.id:", login.login_id)
        print("login.auth_url:", login.auth_url)
        print("login.cancel.status:", canceled.status)
        print("login.completed.success:", completed.success)
        print("account.requires_firefamai_auth:", account.requires_firefamai_auth)


if __name__ == "__main__":
    asyncio.run(main())
