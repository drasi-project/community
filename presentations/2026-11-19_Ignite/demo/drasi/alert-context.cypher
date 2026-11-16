MATCH (a:ignite_alerts)
RETURN a.id AS id, a.status AS status, a.title AS title, a.message AS message,
       a.event_ms AS event_ms, a.received_ms AS received_ms
