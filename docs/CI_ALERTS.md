# CI failure and recovery alerts

Each GigPies/SHR repository's `CI alerts` workflow watches its main CI workflow.
On the first failed push run for the current default-branch tip, it opens a
`CI failed: repository / branch` issue and mentions `PaolaShultz`. Repeated failures
update that incident without adding comments or opening another issue. When CI
passes for the current tip, the bot posts **Recovered — CI is passing again**,
mentions the recipient, changes the title to `CI recovered`, and closes the issue.
A later failure starts a new incident. Each message links the exact run and commit.

GitHub issue email/web delivery follows the recipient's
[notification settings](https://github.com/settings/notifications), including
participating/@mention notifications. This automation cannot recall old emails
or change account notification preferences. If duplicate native Actions failure
emails are unwanted, set **System → Actions → Don't notify**, retaining email
notifications for participating conversations. The bot's failure and recovery
then share one issue thread. Existing run history and logs remain available.

Only `.github/workflows/ci.yml` push runs on the default branch are tracked.
PR/fork runs, other workflows and ordinary successes without an open incident
produce no alerts. Cancelled, skipped, neutral, queued, running or missing CI
never counts as recovery. A green run for an old commit cannot close an incident
for a newer branch tip. Historical failures already followed by success are not
re-announced when installing this workflow.

The notifier re-reads the latest run and branch tip immediately before writing.
It describes that observation; a subsequent push may of course introduce a new
failure. Notifications serialize per repository without cancelling an in-progress
notification. Replayed completions and retries after posting a recovery comment
are idempotent. A manually closed incident stays closed for that run/attempt;
a new failed run can open another incident. Do not edit the hidden bot metadata.
API failures fail visibly instead of claiming notification delivery.

## Implementation and verification

The shared composite action lives in `.github/actions/ci-alerts/`. Consumers pin
its full GigPies commit SHA. It uses only Python's standard library and GitHub's
REST API, with repository-scoped `actions: read`, `contents: read`, `issues: write`.
It does not check out or execute the triggering commit, download artifacts, read
logs, install packages, use a personal token, send SMTP mail or touch hardware.
The repository must have Issues enabled. The ordinary CI workflow retains its
existing permissions and checks. No alert workflow watches itself.

Run the synthetic lifecycle/API regression suite with:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover -s scripts -p test_ci_alerts.py
```

`CI alerts` also supports manual `workflow_dispatch` to reconcile the current
real CI result, e.g. retrying after an API outage. It cannot fabricate a failure
or recovery. Local tests fake the API; live dispatch validates GitHub permissions
and execution without deliberately breaking application CI. Hosted event delivery
and the recipient's email inbox are distinct from the synthetic lifecycle tests.

References: [workflow_run events](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows#workflow_run),
[workflow-run REST API](https://docs.github.com/en/rest/actions/workflow-runs),
[notification settings](https://docs.github.com/en/subscriptions-and-notifications/how-tos/managing-github-actions-notifications).
