MATCH (c:SupportOverdue)-[:OVERDUE_SCENARIO]->(s:ignite_scenario)
WHERE c.run_epoch = s.run_epoch
WITH s, toInteger(c.overdue) AS overdue,
     drasi.previousDistinctValue(CASE WHEN c.overdue > 0 THEN 'overdue' ELSE toString(toInteger(c.next_deadline)) END) AS clear_until,
     drasi.trueFor(c.overdue > 0, 3000) AS overdue_3s
WHERE overdue_3s AND clear_until IS NOT NULL AND clear_until <> 'overdue'
  AND toInteger(clear_until) < 9007199254740991
  AND NOT drasi.trueLater(true, toInteger(clear_until) + 60000)
RETURN 'overdue-' + clear_until AS id, s.run_epoch AS run_epoch, toInteger(clear_until) AS at_ms, '' AS root_id,
       'Support requests overdue' AS title,
       CASE WHEN overdue = 1 THEN '1 customer-support request is past its 15-second deadline.'
            ELSE toString(overdue) + ' customer-support requests are past their 15-second deadline.' END
         + ' More follow until support can run somewhere.' AS text,
       toString(overdue) AS overdue, '15 s' AS deadline
