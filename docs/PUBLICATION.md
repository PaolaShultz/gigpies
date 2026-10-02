# Publication boundaries

Keep original recordings, user settings, private session manifests, source archives,
generated audio, plots and one-off runners in ignored local directories. `user/`,
`artifacts/`, `artefacts/`, `recordings`, `archive/local/` and `sessions/local/` are
private. The `recordings` compatibility link must remain untracked. Ignoring data
preserves it locally; it does not delete it or make already tracked data private.

## Checks before commit and push

Install the versioned hooks in each checkout:

```sh
git config --local core.hooksPath .githooks
```

Check an existing hook configuration before replacing it; compose existing hooks
when needed. These hooks are enabled in the release workstation checkout. Fresh
clones need this local setup because Git does not automatically install hooks.

The pre-commit hook runs `scripts/check_publication.py` against the **complete Git
index**, so force-added files and staged content different from the working file
are checked. The pre-push hook checks every outgoing commit, including a leak that
was deleted before the branch tip. New refs check their complete reachable history.
CI runs the same index check, plus the guard's normal synthetic tests.

The guard rejects private/generated path components, audio/video/archive formats,
unreviewed binary content, symlinks/submodules, common credential signatures and
files above the review size limit. `scripts/publication-policy.json` lists reviewed
scripts/hooks and original binary artwork. Adding a new script requires an explicit
review and policy update; it cannot silently enter through `git add .`.

```sh
git diff --cached --name-status
git diff --cached --stat
git diff --cached --check
python3 scripts/check_publication.py
```

Inspect the actual staged diff as well. Keep `Cargo.toml` and `Cargo.lock` versions
consistent and run the required production checks before publication. Source-only
Git tags/archives contain tracked files; avoid uploading the workspace or its local
artifact directories as release attachments.

Hooks can be bypassed, and pattern checks cannot recognize every secret or private
document. CI runs after upload and cannot undo a disclosure. Human staged-content
review remains required. Do not weaken the policy just to admit generated evidence;
move that evidence to an ignored location. If sensitive data ever enters a published
commit, deleting the tip file does not remove it from history.

## Script ownership

Reusable commands and their synthetic tests belong under `scripts/`; see its
[catalogue](../scripts/README.md). One-off song runners, machine-specific settings,
download helpers and evidence exports belong under ignored `artifacts/` or `user/`.
Normal tests never play audio, download recordings or change host audio settings.
Optional playback helpers require explicit invocation and session authorization.
