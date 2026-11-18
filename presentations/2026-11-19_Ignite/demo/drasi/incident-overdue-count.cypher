MATCH (t:ai_requests)-[:REQUEST_CONFIG]->(s:routing_config)
WHERE t.run_epoch = s.run_epoch AND t.workload = 'support'
RETURN s.run_epoch AS run_epoch,
       sum(CASE WHEN t.completed_ms IS NULL AND t.cancelled_ms IS NULL
                 AND drasi.trueLater(true, t.deadline_ms) THEN 1 ELSE 0 END) AS overdue,
       min(CASE WHEN t.completed_ms IS NULL AND t.cancelled_ms IS NULL
                 AND NOT drasi.trueLater(true, t.deadline_ms) THEN t.deadline_ms
                ELSE 9007199254740991 END) AS next_deadline
