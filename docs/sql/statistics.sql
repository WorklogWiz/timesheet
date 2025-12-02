-- Statistics queries for v2 schema (post-migration)
-- Note: Components have been replaced with tags (JSON arrays)

-- =============================================================================
-- Hours per issue tag (replaces component statistics)
-- =============================================================================
-- This query extracts tags from the JSON array and aggregates time by tag
SELECT
    tags_json.value AS tag,
    SUM(worklogs.time_spent_seconds) / 3600 as hours,
    SUM(worklogs.time_spent_seconds) % 3600 / 60 AS minutes
FROM issues
JOIN worklogs ON issues.issue_key = worklogs.issue_key
CROSS JOIN json_each(issues.tags) AS tags_json
WHERE worklogs.stopped_at IS NOT NULL  -- Only count completed worklogs
GROUP BY tags_json.value
ORDER BY hours DESC, minutes DESC;

-- =============================================================================
-- Total hours spent (all worklogs)
-- =============================================================================
SELECT
    SUM(time_spent_seconds) AS seconds,
    SUM(time_spent_seconds) / 3600 AS hours,
    SUM(time_spent_seconds) % 3600 / 60 AS minutes
FROM worklogs
WHERE stopped_at IS NOT NULL;  -- Only count completed worklogs

-- =============================================================================
-- Worklogs for specific issues today
-- =============================================================================
SELECT *
FROM worklogs
WHERE issue_key IN ('KT-1892', 'KT-2759')
  AND DATE(started_at) = DATE('now');

-- =============================================================================
-- Hours per tag for today
-- =============================================================================
SELECT
    tags_json.value AS tag,
    SUM(time_spent_seconds) / 3600 AS hours,
    SUM(time_spent_seconds) % 3600 / 60 AS minutes
FROM issues
JOIN worklogs ON issues.issue_key = worklogs.issue_key
CROSS JOIN json_each(issues.tags) AS tags_json
WHERE DATE(worklogs.started_at) = DATE('now')
  AND worklogs.stopped_at IS NOT NULL
-- Uncomment to filter by specific tag:
-- AND tags_json.value = 'Booking-general'
GROUP BY tag
ORDER BY tag;

-- =============================================================================
-- Issues without tags (replaces "issues without components")
-- =============================================================================
SELECT issue_key, summary
FROM issues
WHERE json_array_length(tags) = 0
   OR tags IS NULL
   OR tags = '[]';

-- =============================================================================
-- Total hours for today
-- =============================================================================
SELECT
    SUM(time_spent_seconds) AS seconds,
    SUM(time_spent_seconds) / 3600 AS hours,
    SUM(time_spent_seconds) % 3600 / 60 AS minutes
FROM worklogs
WHERE DATE(started_at) = DATE('now')
  AND stopped_at IS NOT NULL;

-- =============================================================================
-- List all unique tags
-- =============================================================================
SELECT DISTINCT tags_json.value AS tag
FROM issues
CROSS JOIN json_each(issues.tags) AS tags_json
ORDER BY tag;

-- =============================================================================
-- Hours per issue (with issue summary)
-- =============================================================================
SELECT
    i.issue_key,
    i.summary,
    SUM(w.time_spent_seconds) / 3600 AS hours,
    SUM(w.time_spent_seconds) % 3600 / 60 AS minutes
FROM issues i
JOIN worklogs w ON i.issue_key = w.issue_key
WHERE w.stopped_at IS NOT NULL
GROUP BY i.issue_key, i.summary
ORDER BY hours DESC, minutes DESC;

-- =============================================================================
-- Active timer (if any)
-- =============================================================================
SELECT
    w.id,
    w.issue_key,
    i.summary,
    w.started_at,
    w.comment,
    ROUND((JULIANDAY('now') - JULIANDAY(w.started_at)) * 24, 2) AS hours_elapsed
FROM worklogs w
LEFT JOIN issues i ON w.issue_key = i.issue_key
WHERE w.stopped_at IS NULL
ORDER BY w.started_at DESC;

-- =============================================================================
-- Recent activity (last 7 days)
-- =============================================================================
SELECT
    DATE(started_at) AS date,
    COUNT(*) AS entries,
    SUM(time_spent_seconds) / 3600 AS hours,
    SUM(time_spent_seconds) % 3600 / 60 AS minutes
FROM worklogs
WHERE started_at >= DATE('now', '-7 days')
  AND stopped_at IS NOT NULL
GROUP BY DATE(started_at)
ORDER BY date DESC;

-- =============================================================================
-- Sync status summary
-- =============================================================================
SELECT
    synced_to_provider,
    COUNT(*) AS count,
    SUM(time_spent_seconds) / 3600 AS hours
FROM worklogs
WHERE stopped_at IS NOT NULL
GROUP BY synced_to_provider;

-- =============================================================================
-- Work patterns by day of week
-- =============================================================================
SELECT
    CASE CAST(STRFTIME('%w', started_at) AS INTEGER)
        WHEN 0 THEN 'Sunday'
        WHEN 1 THEN 'Monday'
        WHEN 2 THEN 'Tuesday'
        WHEN 3 THEN 'Wednesday'
        WHEN 4 THEN 'Thursday'
        WHEN 5 THEN 'Friday'
        WHEN 6 THEN 'Saturday'
    END AS day_of_week,
    COUNT(*) AS entries,
    SUM(time_spent_seconds) / 3600 AS hours
FROM worklogs
WHERE stopped_at IS NOT NULL
  AND started_at >= DATE('now', '-30 days')
GROUP BY STRFTIME('%w', started_at)
ORDER BY CAST(STRFTIME('%w', started_at) AS INTEGER);
