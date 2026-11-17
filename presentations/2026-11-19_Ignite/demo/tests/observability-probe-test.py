"""Run in the probe image; no network, real telemetry or external services needed."""
import json
from pathlib import Path
import sqlite3
import tempfile
import unittest

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

    def test_notification_is_recorded_once(self):
        first = probe.record_webhook(self.payload)
        second = probe.record_webhook(self.payload)
        self.assertEqual(first, second)
        self.assertEqual(json.loads(self.row()[0]), self.payload)


if __name__ == "__main__":
    unittest.main()
