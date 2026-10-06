"""Reconcile a default-branch CI incident using GitHub metadata only."""

import json
import os
import re
from pathlib import Path
from urllib.error import HTTPError
from urllib.parse import quote, urlencode
from urllib.request import Request, urlopen

BOT = "github-actions[bot]"
FAILURES = {"failure", "timed_out", "action_required", "startup_failure"}


class GitHub:
    def __init__(self, repository, token):
        self.base = "https://api.github.com/repos/" + repository
        self.token = token

    def request(self, method, path, data=None):
        request = Request(
            self.base + path,
            data=None if data is None else json.dumps(data).encode(),
            method=method,
            headers={
                "Authorization": "Bearer " + self.token,
                "Accept": "application/vnd.github+json",
                "X-GitHub-Api-Version": "2022-11-28",
                "Content-Type": "application/json",
                "User-Agent": "gigpies-ci-alerts",
            },
        )
        try:
            with urlopen(request, timeout=30) as response:
                return json.load(response)
        except HTTPError as error:
            # Never log headers, token or arbitrary remote response content.
            raise RuntimeError(f"GitHub {method} {path}: HTTP {error.code}") from None

    def pages(self, path):
        separator = "&" if "?" in path else "?"
        for page in range(1, 101):
            items = self.request("GET", f"{path}{separator}per_page=100&page={page}")
            yield from items
            if len(items) < 100:
                return
        raise RuntimeError("Pagination limit reached; refusing an incomplete reconciliation")


def position(run):
    return run["run_number"], run["run_attempt"]


def snapshot(api, workflow, branch):
    query = urlencode({"branch": branch, "event": "push", "per_page": 1})
    runs = api.request("GET", f"/actions/workflows/{workflow}/runs?{query}")["workflow_runs"]
    head = api.request("GET", "/branches/" + quote(branch, safe=""))["commit"]["sha"]
    if not runs:
        return None
    run = runs[0]
    if run["head_sha"] != head or run["status"] != "completed":
        return None
    return run


def identity(run):
    return run["id"], run["run_attempt"], run["head_sha"], run["conclusion"]


def reconcile(api, repository, branch, recipient, event_name, event):
    workflow = api.request("GET", "/actions/workflows/ci.yml")["id"]
    if event_name == "workflow_run":
        trigger = event["workflow_run"]
        if (trigger["workflow_id"] != workflow or trigger["event"] != "push"
                or trigger["head_branch"] != branch
                or trigger["head_repository"]["full_name"] != repository):
            return "Ignored non-default-branch push event"
    elif event_name != "workflow_dispatch":
        return "Ignored unsupported event"

    run = snapshot(api, workflow, branch)
    if run is None:
        return "Current default-branch CI is pending or absent; no notification"
    if run["conclusion"] not in FAILURES | {"success"}:
        return "Cancelled, skipped or neutral CI is not recovery"
    marker = f"<!-- gigpies-ci-incident-v1:{workflow}:{branch} -->"
    incidents = [issue for issue in api.pages("/issues?state=all&creator=github-actions%5Bbot%5D")
                 if issue.get("user", {}).get("login") == BOT
                 and not issue.get("pull_request")
                 and (issue.get("body") or "").startswith(marker + "\n")]
    incident = max(incidents, key=lambda issue: issue["number"], default=None)
    if incident:
        saved = re.search(r"<!-- ci-position:(\d+):(\d+) -->", incident["body"])
        if saved is None:
            raise RuntimeError("Incident metadata is missing; preserve it for review")
        previous = tuple(map(int, saved.groups()))
        if position(run) < previous:
            return "Ignored older run"
        if incident["state"] == "closed" and position(run) <= previous:
            return "Incident already closed; no notification"

    # Re-read immediately before writing: a newer push/run must not be reported fixed.
    current = snapshot(api, workflow, branch)
    if current is None or identity(current) != identity(run):
        return "CI changed during reconciliation; no notification"
    link = f"https://github.com/{repository}/actions/runs/{run['id']}"
    details = (f"[CI run #{run['run_number']}, attempt {run['run_attempt']}]({link})"
               f" for commit `{run['head_sha']}` on `{branch}`")
    metadata = marker + f"\n<!-- ci-position:{run['run_number']}:{run['run_attempt']} -->\n"
    title = f"CI failed: {repository} / {branch}"
    body = (metadata + f"\n@{recipient} **CI failed** ({run['conclusion']}).\n\n{details}.\n\n"
            "This incident stays open through repeated failures. A passing CI run for the "
            "current branch tip will post **Recovered** and close it. Older red runs remain "
            "historical evidence; use this thread for the incident's resolution.\n")
    if run["conclusion"] in FAILURES:
        if incident and incident["state"] == "open":
            if incident["body"] != body or incident["title"] != title:
                api.request("PATCH", f"/issues/{incident['number']}", {"title": title, "body": body})
            return "Existing failure incident updated without another comment"
        api.request("POST", "/issues", {"title": title, "body": body})
        return "Failure incident opened"
    if not incident or incident["state"] != "open":
        return "CI passed; no unresolved incident"
    number = incident["number"]
    recovery_marker = f"<!-- ci-recovered:{run['id']}:{run['run_attempt']} -->"
    comments = api.pages(f"/issues/{number}/comments")
    if not any(comment.get("user", {}).get("login") == BOT
               and recovery_marker in (comment.get("body") or "") for comment in comments):
        api.request("POST", f"/issues/{number}/comments", {
            "body": f"{recovery_marker}\n\n@{recipient} **Recovered — CI is passing again.**\n\n"
                    f"{details}. The earlier failure is resolved by this passing result.\n"
        })
    api.request("PATCH", f"/issues/{number}", {
        "state": "closed", "state_reason": "completed",
        "title": f"CI recovered: {repository} / {branch}",
        "body": metadata + f"\n**Recovered.** {details} passed.\n\n"
                + "## Earlier failure\n" + incident["body"].split("\n", 2)[2],
    })
    return "Recovery posted and incident closed"


def main():
    repository = os.environ["GITHUB_REPOSITORY"]
    recipient = os.environ["CI_ALERT_RECIPIENT"]
    if not re.fullmatch(r"[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+", repository):
        raise ValueError("Invalid repository")
    if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9-]{0,38}", recipient):
        raise ValueError("Invalid notification recipient")
    event = json.loads(Path(os.environ["GITHUB_EVENT_PATH"]).read_text())
    api = GitHub(repository, os.environ["CI_ALERT_TOKEN"])
    branch = api.request("GET", "")["default_branch"]
    print(reconcile(api, repository, branch, recipient, os.environ["GITHUB_EVENT_NAME"], event))


if __name__ == "__main__":
    main()
