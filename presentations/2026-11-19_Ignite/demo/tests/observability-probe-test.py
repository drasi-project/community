"""Run in the probe image; no network, real telemetry or external services needed."""
import json
import os
from pathlib import Path
import sqlite3
import tempfile
import threading
import unittest
from unittest.mock import patch, MagicMock
import urllib.error

import probe


class WebhookTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.old_database = probe.DATABASE
        probe.DATABASE = Path(self.temp.name) / "webhooks.sqlite"
        with sqlite3.connect(probe.DATABASE) as db:
            db.execute("""CREATE TABLE webhooks (
                id INTEGER PRIMARY KEY, digest TEXT UNIQUE, received_at REAL,
                payload TEXT NOT NULL, forwarded_at REAL, delivery_error TEXT)""")
        self.payload = {"groupKey": "probe", "status": "firing", "commonLabels": {"alertname": "probe"}}

    def tearDown(self):
        probe.DATABASE = self.old_database
        self.temp.cleanup()

    def row(self):
        with sqlite3.connect(probe.DATABASE) as db:
            return db.execute("SELECT payload, forwarded_at, delivery_error FROM webhooks").fetchone()

    @patch.dict(os.environ, {"DRASI_ALERT_URL": ""})
    def test_capture_is_not_claimed_as_forwarded(self):
        first, error = probe.record_webhook(self.payload)
        second, _ = probe.record_webhook(self.payload)
        self.assertEqual(first, second)
        self.assertIsNone(error)
        self.assertEqual(json.loads(self.row()[0]), self.payload)
        self.assertIsNone(self.row()[1])

    @patch.dict(os.environ, {"DRASI_ALERT_URL": "http://127.0.0.1:18422/alerts/grafana"})
    @patch("probe.urllib.request.urlopen")
    def test_success_forwards_unchanged_once(self, urlopen):
        urlopen.return_value = MagicMock()
        probe.record_webhook(self.payload)
        probe.record_webhook(self.payload)
        self.assertEqual(urlopen.call_count, 1)
        self.assertEqual(json.loads(urlopen.call_args.args[0].data), self.payload)
        self.assertIsNotNone(self.row()[1])
        self.assertIsNone(self.row()[2])

    @patch.dict(os.environ, {"DRASI_ALERT_URL": "http://127.0.0.1:18422/alerts/grafana"})
    @patch("probe.urllib.request.urlopen")
    def test_concurrent_duplicate_waits_for_first_delivery(self, urlopen):
        entered = threading.Event()
        release = threading.Event()
        def delayed_response(*_args, **_kwargs):
            entered.set()
            self.assertTrue(release.wait(2))
            return MagicMock()
        urlopen.side_effect = delayed_response
        first = threading.Thread(target=probe.record_webhook, args=(self.payload,))
        second = threading.Thread(target=probe.record_webhook, args=(self.payload,))
        first.start()
        self.assertTrue(entered.wait(2))
        second.start()
        release.set()
        first.join(3)
        second.join(3)
        self.assertFalse(first.is_alive())
        self.assertFalse(second.is_alive())
        self.assertEqual(urlopen.call_count, 1)

    @patch.dict(os.environ, {"DRASI_ALERT_URL": "http://127.0.0.1:18422/alerts/grafana"})
    @patch("probe.urllib.request.urlopen")
    def test_delivery_error_remains_visible_and_can_retry(self, urlopen):
        urlopen.side_effect = urllib.error.URLError("native route unavailable")
        event_id, error = probe.record_webhook(self.payload)
        self.assertIn("native route unavailable", error)
        self.assertIsNone(self.row()[1])
        self.assertIn("native route unavailable", self.row()[2])
        urlopen.side_effect = None
        urlopen.return_value = MagicMock()
        retried_id, error = probe.record_webhook(self.payload)
        self.assertEqual(event_id, retried_id)
        self.assertIsNone(error)
        self.assertIsNotNone(self.row()[1])
        self.assertIsNone(self.row()[2])


if __name__ == "__main__":
    unittest.main()
