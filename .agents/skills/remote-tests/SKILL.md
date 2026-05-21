---
name: remote-tests
description: How to run tests using remote executor.
---

Some firefam integration tests support a running against a remote executor.
This means that when FIREFAM_TEST_REMOTE_ENV environment variable is set they will attempt to start an executor process in a docker container FIREFAM_TEST_REMOTE_ENV points to and use it in tests.

Docker container is built and initialized via ./scripts/test-remote-env.sh

Currently running remote tests is only supported on Linux, so you need to use a devbox to run them

You can list devboxes via `applied_devbox ls`, pick the one with `firefam` in the name.
Connect to devbox via `ssh <devbox_name>`.
Reuse the same checkout of firefam in `~/code/firefam`. Reset files if needed. Multiple checkouts take longer to build and take up more space.
Check whether the SHA and modified files are in sync between remote and local.
