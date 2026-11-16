#!/usr/bin/env python3
"""Regression checks for the smoke gate's snapshot/delta and cycle semantics."""
from pathlib import Path
import runpy
import time
import unittest

AlertSubscription = runpy.run_path(str(Path(__file__).with_name("observability-smoke.py")))["AlertSubscription"]


class QueryGateTests(unittest.TestCase):
    def setUp(self):
        self.subscription = AlertSubscription()
        self.notification = {"groupKey": "group-1", "status": "firing", "title": "test alert", "message": "actual payload"}
        self.row = {"id": "group-1", "status": "firing", "title": "test alert", "message": "actual payload"}

    def test_existing_snapshot_does_not_prove_new_cycle(self):
        self.subscription.consume("snapshot", [self.row])
        self.assertTrue(self.subscription.ready())
        self.assertIsNone(self.subscription.matching(self.notification, 0))

    def test_matching_add_requires_current_identity_and_cycle(self):
        self.subscription.consume("snapshot", [])
        before = time.monotonic()
        self.subscription.consume("message", {"results": [{"type": "ADD", "data": self.row}]})
        self.assertEqual(self.subscription.matching(self.notification, before)["row"], self.row)
        self.assertIsNone(self.subscription.matching({**self.notification, "groupKey": "other"}, before))
        self.assertIsNone(self.subscription.matching({**self.notification, "message": "other cycle"}, before))
        self.assertIsNone(self.subscription.matching(self.notification, time.monotonic()))

    def test_update_after_state_proves_resolution(self):
        before = time.monotonic()
        after = {**self.row, "status": "resolved"}
        self.subscription.consume("message", {"results": [{"type": "UPDATE", "before": self.row, "after": after}]})
        self.assertEqual(
            self.subscription.matching({**self.notification, "status": "resolved"}, before)["row"],
            after,
        )
        self.assertIsNone(self.subscription.matching(self.notification, before))

    def test_invalid_or_failed_stream_cannot_pass(self):
        with self.assertRaises(ValueError):
            self.subscription.consume("snapshot", {"not": "an array"})
        self.subscription.error = "subscription disconnected"
        with self.assertRaisesRegex(RuntimeError, "disconnected"):
            self.subscription.ready()
        with self.assertRaisesRegex(RuntimeError, "disconnected"):
            self.subscription.matching(self.notification, 0)


if __name__ == "__main__":
    unittest.main()
