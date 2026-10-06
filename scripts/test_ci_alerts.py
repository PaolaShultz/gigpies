"""Synthetic GitHub API regressions; never send real notifications."""
import copy
import importlib.util
from pathlib import Path
import unittest

SOURCE = Path(__file__).resolve().parents[1] / '.github/actions/ci-alerts/notify.py'
spec = importlib.util.spec_from_file_location('ci_alerts', SOURCE)
alerts = importlib.util.module_from_spec(spec)
spec.loader.exec_module(alerts)


def run(number=1, conclusion='failure', attempt=1, sha='a' * 40):
    return dict(id=100 + number, run_number=number, run_attempt=attempt,
                head_sha=sha, status='completed', conclusion=conclusion)


class Fake:
    def __init__(self):
        self.run = run()
        self.head = self.run['head_sha']
        self.issues = []
        self.comments = []
        self.writes = []
        self.fail_close = False
        self.advance = False
        self.reads = 0

    def request(self, method, path, data=None):
        if method == 'GET':
            if path == '/actions/workflows/ci.yml':
                return {'id': 42}
            if path.startswith('/actions/workflows/42/runs?'):
                self.reads += 1
                if self.advance and self.reads % 2 == 0:
                    self.head = 'b' * 40
                return {'workflow_runs': [copy.deepcopy(self.run)] if self.run else []}
            if path == '/branches/main':
                return {'commit': {'sha': self.head}}
            raise AssertionError(path)
        self.writes.append((method, path, copy.deepcopy(data)))
        if method == 'POST' and path == '/issues':
            self.issues.append(dict(number=len(self.issues) + 1, user={'login': alerts.BOT},
                                    state='open', **data))
        elif method == 'POST' and path.endswith('/comments'):
            self.comments.append(dict(user={'login': alerts.BOT}, **data))
        elif method == 'PATCH':
            if self.fail_close and data.get('state') == 'closed':
                raise RuntimeError('simulated interruption after comment')
            self.issues[int(path.split('/')[-1]) - 1].update(data)
        else:
            raise AssertionError((method, path))

    def pages(self, path):
        return copy.deepcopy(self.comments if path.endswith('/comments') else self.issues)

    def reconcile(self, event_name='workflow_dispatch', event=None):
        return alerts.reconcile(self, 'owner/repo', 'main', 'owner', event_name, event or {})


class AlertsTests(unittest.TestCase):
    def test_failure_repeat_recovery_and_new_incident(self):
        api = Fake()
        api.reconcile()
        self.assertEqual(len(api.issues), 1)
        self.assertIn('@owner **CI failed**', api.issues[0]['body'])
        writes = len(api.writes)
        api.reconcile()
        self.assertEqual(len(api.writes), writes)
        api.run = run(2)
        api.reconcile()
        self.assertEqual(len(api.issues), 1)
        self.assertEqual(api.comments, [])
        api.run = run(3, 'success')
        api.reconcile()
        self.assertEqual(api.issues[0]['state'], 'closed')
        self.assertTrue(api.issues[0]['title'].startswith('CI recovered:'))
        self.assertIn('@owner **Recovered', api.comments[0]['body'])
        writes = len(api.writes)
        api.reconcile()
        self.assertEqual(len(api.writes), writes)
        api.run = run(4)
        api.reconcile()
        self.assertEqual(len(api.issues), 2)

    def test_green_install_does_not_resurrect_historical_failures(self):
        api = Fake()
        api.run = run(4, 'success')
        api.reconcile()
        self.assertEqual(api.writes, [])

    def test_recovery_retries_comment_once_after_partial_failure(self):
        api = Fake()
        api.reconcile()
        api.run = run(2, 'success')
        api.fail_close = True
        with self.assertRaises(RuntimeError):
            api.reconcile()
        self.assertEqual(len(api.comments), 1)
        api.fail_close = False
        api.reconcile()
        self.assertEqual(len(api.comments), 1)
        self.assertEqual(api.issues[0]['state'], 'closed')

    def test_rerun_recovery_and_old_result_cannot_reopen(self):
        api = Fake()
        api.reconcile()
        api.run = run(1, 'success', attempt=2)
        api.reconcile()
        self.assertEqual(api.issues[0]['state'], 'closed')
        api.run = run(1, 'failure', attempt=1)
        count = len(api.writes)
        api.reconcile()
        self.assertEqual(len(api.writes), count)

    def test_pending_absent_and_cancelled_never_close_incident(self):
        for mode in ['queued', 'in_progress', 'cancelled', 'skipped', 'neutral', 'absent']:
            with self.subTest(mode=mode):
                api = Fake()
                api.reconcile()
                api.run = run(2, mode)
                if mode in ['queued', 'in_progress']:
                    api.run['status'] = mode
                if mode == 'absent':
                    api.run = None
                count = len(api.writes)
                api.reconcile()
                self.assertEqual(len(api.writes), count)
                self.assertEqual(api.issues[0]['state'], 'open')

    def test_success_at_old_head_and_concurrent_push_do_not_recover(self):
        for advance in [False, True]:
            api = Fake()
            api.reconcile()
            api.run = run(2, 'success')
            api.advance = advance
            if not advance:
                api.head = 'b' * 40
            count = len(api.writes)
            api.reconcile()
            self.assertEqual(len(api.writes), count)

    def test_non_ci_pr_fork_and_non_default_branch_events_ignored(self):
        base = dict(workflow_id=42, event='push', head_branch='main',
                    head_repository={'full_name': 'owner/repo'})
        for change in [dict(workflow_id=99), dict(event='pull_request'),
                       dict(head_branch='topic'), dict(head_repository={'full_name': 'fork/repo'})]:
            api = Fake()
            api.reconcile('workflow_run', {'workflow_run': base | change})
            self.assertEqual(api.writes, [])
        api = Fake()
        api.reconcile('workflow_run', {'workflow_run': base})
        self.assertEqual(len(api.issues), 1)

    def test_manual_close_is_respected_until_another_failure(self):
        api = Fake()
        api.reconcile()
        api.issues[0]['state'] = 'closed'
        count = len(api.writes)
        api.reconcile()
        self.assertEqual(len(api.writes), count)
        api.run = run(2)
        api.reconcile()
        self.assertEqual(len(api.issues), 2)

    def test_user_issue_and_other_workflow_are_not_modified(self):
        api = Fake()
        api.issues = [dict(number=1, user={'login': 'owner'}, state='open', title='Mine',
                           body='<!-- gigpies-ci-incident-v1:42:main -->\n'),
                      dict(number=2, user={'login': alerts.BOT}, state='open', title='Other',
                           body='<!-- gigpies-ci-incident-v1:99:main -->\n')]
        before = copy.deepcopy(api.issues)
        api.reconcile()
        self.assertEqual(api.issues[:2], before)
        self.assertEqual(len(api.issues), 3)

    def test_pagination_finds_incident_beyond_first_page(self):
        class Pages(alerts.GitHub):
            def __init__(self):
                self.paths = []
            def request(self, method, path, data=None):
                self.paths.append(path)
                return list(range(100)) if path.endswith('&page=1') else [101]
        api = Pages()
        self.assertEqual(len(list(api.pages('/issues?state=all'))), 101)
        self.assertEqual(len(api.paths), 2)

    def test_issue_api_error_is_not_reported_as_success(self):
        class Denied(Fake):
            def request(self, method, path, data=None):
                if method == 'POST':
                    raise RuntimeError('403')
                return super().request(method, path, data)
        with self.assertRaisesRegex(RuntimeError, '403'):
            Denied().reconcile()


if __name__ == '__main__':
    unittest.main()
