"""Publication checks against real temporary Git indexes and commit histories."""
import json
from pathlib import Path
import subprocess
import tempfile
import unittest

import check_publication


class PublicationTests(unittest.TestCase):
    def setUp(self):
        temp = tempfile.TemporaryDirectory()
        self.addCleanup(temp.cleanup)
        self.root = Path(temp.name)
        self.script = Path(check_publication.__file__).resolve()
        self.git('init', '-q')
        self.git('config', 'user.name', 'Synthetic Test')
        self.git('config', 'user.email', 'test@example.invalid')
        self.write('scripts/publication-policy.json', json.dumps({
            'root_files': ['README.md'], 'directories': ['src', 'scripts', 'docs'],
            'scripts': [], 'binary_artwork': [],
        }).encode())
        self.write('src/lib.rs', b'// public code\n')
        self.git('add', '.')

    def git(self, *args):
        return subprocess.check_output(['git', *args], cwd=self.root, stderr=subprocess.PIPE)

    def write(self, path, data):
        file = self.root / path
        file.parent.mkdir(parents=True, exist_ok=True)
        file.write_bytes(data)

    def check(self, *args, stdin=None):
        return subprocess.run(['python3', str(self.script), *args], cwd=self.root,
                              input=stdin, capture_output=True, text=True)

    def test_private_paths_and_unreviewed_scripts_are_blocked_even_when_force_added(self):
        self.assertEqual(self.check().returncode, 0)
        for name in ('user/settings.json', 'docs/Artefacts/results.json', 'scripts/scratch.py', 'scripts/unreviewed'):
            with self.subTest(name=name):
                self.write(name, b'{}')
                if name == 'scripts/unreviewed':
                    (self.root / name).chmod(0o755)
                self.git('add', '-f', name)
                result = self.check()
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(name, result.stderr)
                self.git('rm', '--cached', name)

    def test_staged_secret_is_checked_even_when_worktree_is_clean(self):
        secret = b'gh' + b'p_' + b'A' * 40
        self.write('src/lib.rs', secret)
        self.git('add', 'src/lib.rs')
        self.write('src/lib.rs', b'// safe worktree\n')
        result = self.check()
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('credential-like', result.stderr)
        self.assertNotIn(secret.decode(), result.stderr)

    def test_renamed_audio_and_external_symlink_are_blocked(self):
        self.write('docs/evidence.txt', b'RIFF' + b'\0' * 4 + b'WAVE')
        self.git('add', 'docs/evidence.txt')
        self.assertIn('media/archive signature', self.check().stderr)
        self.git('rm', '--cached', 'docs/evidence.txt')
        (self.root / 'docs/link').symlink_to('/outside/private-recordings')
        self.git('add', 'docs/link')
        self.assertIn('symlink', self.check().stderr)

    def test_push_checks_a_leak_deleted_by_a_later_commit(self):
        self.git('commit', '-qm', 'safe baseline')
        baseline = self.git('rev-parse', 'HEAD').decode().strip()
        self.write('user/private.json', b'{}')
        self.git('add', '-f', 'user/private.json')
        self.git('commit', '-qm', 'simulated private commit')
        self.git('rm', '-q', 'user/private.json')
        self.git('commit', '-qm', 'removed at tip')
        tip = self.git('rev-parse', 'HEAD').decode().strip()
        self.assertEqual(self.check('--revision', tip).returncode, 0)
        result = self.check('--push', stdin=f'refs/heads/main {tip} refs/heads/main {baseline}\n')
        self.assertNotEqual(result.returncode, 0)
        self.assertIn('user/private.json', result.stderr)


if __name__ == '__main__':
    unittest.main()
