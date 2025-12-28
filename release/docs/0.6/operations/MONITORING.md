# RustyDB v0.6.0 - Monitoring and Alerting Guide

**Document Version**: 1.0
**Release**: v0.6.0
**Last Updated**: 2025-12-28
**Classification**: Enterprise Operations

---

## Table of Contents

1. [Monitoring Overview](#monitoring-overview)
2. [Health Monitoring](#health-monitoring)
3. [Performance Metrics](#performance-metrics)
4. [Prometheus Integration](#prometheus-integration)
5. [Grafana Dashboards](#grafana-dashboards)
6. [Alerting Strategy](#alerting-strategy)
7. [Log Monitoring](#log-monitoring)
8. [Application Performance Monitoring](#application-performance-monitoring)
9. [Capacity Monitoring](#capacity-monitoring)
10. [Security Monitoring](#security-monitoring)

---

## Monitoring Overview

### Monitoring Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    RustyDB Instance                      │
│  ┌────────────────────────────────────────────────────┐ │
│  │  Metrics Collector                                 │ │
│  │  - Performance metrics                             │ │
│  │  - Resource utilization                            │ │
│  │  - Health status                                   │ │
│  │  - Security events                                 │ │
│  └────────────┬───────────────────────────────────────┘ │
└───────────────┼─────────────────────────────────────────┘
                │
                ▼
        ┌───────────────┐
        │  Metrics API  │
        │  Port: 9100   │
        │  /metrics     │
        └───────┬───────┘
                │
    ┌───────────┼───────────┐
    │           │           │
    ▼           ▼           ▼
┌──────────┐ ┌────────┐ ┌──────────┐
│Prometheus│ │ Grafana│ │AlertMgr  │
│ (Collect)│ │(Visualize)│ │(Alert)   │
└──────────┘ └────────┘ └──────────┘
```

### Monitoring Components

1. **Metrics Endpoint**: Prometheus-compatible metrics at `/metrics`
2. **Health Endpoint**: JSON health status at `/api/v1/admin/health`
3. **Performance Endpoint**: Real-time stats at `/api/v1/stats/performance`
4. **Logs**: Structured JSON logs for aggregation
5. **Audit Log**: Security event logging

---

## Health Monitoring

### Health Check Endpoint

**URL**: `http://localhost:8080/api/v1/admin/health`

**Example Response**:
```json
{
  "status": "healthy",
  "version": "0.6.0",
  "uptime_seconds": 3600,
  "checks": {
    "database": {
      "status": "healthy",
      "message": "Database is operational",
      "last_check": 1703721600
    },
    "storage": {
      "status": "healthy",
      "message": null,
      "last_check": 1703721600
    },
    "replication": {
      "status": "healthy",
      "message": "All standbys in sync",
      "last_check": 1703721600
    }
  }
}
```

### Health Check Script

```bash
#!/bin/bash
# /usr/local/bin/rustydb-health-check.sh

HEALTH_URL="http://localhost:8080/api/v1/admin/health"
ALERT_EMAIL="dba-team@company.com"

# Get health status
HEALTH=$(curl -s $HEALTH_URL)
STATUS=$(echo $HEALTH | jq -r '.status')

if [ "$STATUS" != "healthy" ]; then
    echo "ALERT: RustyDB health check failed" | mail -s "RustyDB Health Alert" $ALERT_EMAIL
    echo "$HEALTH" | mail -s "RustyDB Health Details" $ALERT_EMAIL
    exit 1
fi

exit 0
```

**Schedule Health Checks**:
```bash
# Add to crontab
*/5 * * * * /usr/local/bin/rustydb-health-check.sh
```

### Component Health Checks

```bash
# Check all components
curl http://localhost:8080/api/v1/admin/health | jq '.checks'

# Check specific component
curl http://localhost:8080/api/v1/admin/health | jq '.checks.database'
```

---

## Performance Metrics

### Key Performance Indicators (KPIs)

#### Database Performance

| Metric | Description | Target | Alert Threshold |
|--------|-------------|--------|----------------|
| Transactions/sec | Query throughput | > 1000 | < 100 |
| Query Latency (p50) | Median response time | < 10ms | > 100ms |
| Query Latency (p95) | 95th percentile | < 50ms | > 500ms |
| Query Latency (p99) | 99th percentile | < 100ms | > 1000ms |
| Cache Hit Ratio | Buffer pool hits | > 95% | < 80% |
| Active Connections | Concurrent connections | 100-500 | > 800 |
| Connection Pool Util | Pool utilization | 50-70% | > 90% |

#### Resource Utilization

| Metric | Description | Target | Warning | Critical |
|--------|-------------|--------|---------|----------|
| CPU Usage | CPU utilization | < 60% | > 75% | > 90% |
| Memory Usage | RAM utilization | < 70% | > 80% | > 90% |
| Disk Usage | Storage utilization | < 70% | > 75% | > 85% |
| Disk I/O (IOPS) | I/O operations | 1000-10000 | > 50000 | > 100000 |
| Network Throughput | Network utilization | < 50% | > 70% | > 90% |

### Performance Metrics Endpoint

**URL**: `http://localhost:8080/api/v1/stats/performance`

**Example Response**:
```json
{
  "cpu_usage_percent": 45.2,
  "memory_usage_bytes": 2147483648,
  "memory_usage_percent": 65.3,
  "disk_io_read_bytes": 1048576000,
  "disk_io_write_bytes": 524288000,
  "cache_hit_ratio": 0.96,
  "transactions_per_second": 1234.5,
  "locks_held": 42,
  "deadlocks": 0
}
```

### Query Performance Monitoring

```sql
-- Top 10 slowest queries
SELECT
    query_id,
    query_text,
    avg_execution_time_ms,
    max_execution_time_ms,
    execution_count,
    total_cpu_time_ms
FROM v$slow_queries
WHERE avg_execution_time_ms > 100
ORDER BY avg_execution_time_ms DESC
LIMIT 10;

-- Active sessions
SELECT
    session_id,
    username,
    query,
    duration_seconds,
    status
FROM v$active_sessions
ORDER BY duration_seconds DESC
LIMIT 20;

-- Lock contention
SELECT
    blocker_session,
    blocked_session,
    lock_type,
    wait_time_seconds
FROM v$lock_waits
ORDER BY wait_time_seconds DESC
LIMIT 10;
```

---

## Prometheus Integration

### Enable Metrics Export

**Configuration** (`conf/rustydb.toml`):
```toml
[metrics]
enabled = true
mode = "pull"
listen_host = "0.0.0.0"
listen_port = 9100
path = "/metrics"
```

### Prometheus Configuration

**prometheus.yml**:
```yaml
global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  - job_name: 'rustydb'
    static_configs:
      - targets:
        - 'localhost:9100'
        labels:
          instance: 'prod-primary'
          environment: 'production'

  - job_name: 'rustydb-cluster'
    static_configs:
      - targets:
        - 'node1.example.com:9100'
        - 'node2.example.com:9100'
        - 'node3.example.com:9100'
        labels:
          cluster: 'prod-cluster'
```

### Key Prometheus Metrics

```
# Database metrics
rustydb_transactions_total
rustydb_queries_total
rustydb_query_duration_seconds
rustydb_cache_hit_ratio
rustydb_active_connections
rustydb_connection_pool_size
rustydb_connection_pool_active
rustydb_connection_pool_idle

# Resource metrics
rustydb_cpu_usage_percent
rustydb_memory_usage_bytes
rustydb_memory_usage_percent
rustydb_disk_usage_bytes
rustydb_disk_io_read_bytes_total
rustydb_disk_io_write_bytes_total
rustydb_network_rx_bytes_total
rustydb_network_tx_bytes_total

# Storage metrics
rustydb_buffer_pool_pages_total
rustydb_buffer_pool_pages_dirty
rustydb_buffer_pool_hit_ratio
rustydb_wal_writes_total
rustydb_wal_bytes_total

# Replication metrics (if applicable)
rustydb_replication_lag_seconds
rustydb_replication_bytes_lag

# Security metrics
rustydb_failed_logins_total
rustydb_threats_detected_total
```

### Sample Prometheus Queries

```promql
# Average query latency over 5 minutes
rate(rustydb_query_duration_seconds_sum[5m]) / rate(rustydb_query_duration_seconds_count[5m])

# Cache hit ratio
rustydb_cache_hit_ratio

# Connection pool utilization
(rustydb_connection_pool_active / rustydb_connection_pool_size) * 100

# Replication lag (seconds)
rustydb_replication_lag_seconds

# Failed login rate
rate(rustydb_failed_logins_total[5m])
```

---

## Grafana Dashboards

### Dashboard Setup

1. **Install Grafana**:
```bash
# Ubuntu/Debian
sudo apt-get install -y grafana

# Start service
sudo systemctl start grafana-server
sudo systemctl enable grafana-server
```

2. **Access Grafana**: `http://localhost:3000` (default admin/admin)

3. **Add Prometheus Data Source**:
   - Configuration → Data Sources → Add data source
   - Select Prometheus
   - URL: `http://localhost:9090`
   - Save & Test

### RustyDB Dashboard Panels

#### 1. Overview Dashboard

**Panels**:
- Database uptime
- Current TPS
- Active connections
- Cache hit ratio
- CPU usage
- Memory usage
- Disk usage

**Example Panel (TPS)**:
```json
{
  "title": "Transactions Per Second",
  "targets": [
    {
      "expr": "rate(rustydb_transactions_total[1m])",
      "legendFormat": "TPS"
    }
  ],
  "type": "graph"
}
```

#### 2. Performance Dashboard

**Panels**:
- Query latency (p50, p95, p99)
- Query throughput
- Slow queries count
- Lock wait time
- Buffer pool hit ratio
- I/O operations

#### 3. Resource Dashboard

**Panels**:
- CPU usage over time
- Memory usage over time
- Disk I/O (read/write)
- Network I/O
- Disk space usage
- Connection pool usage

#### 4. Replication Dashboard

**Panels**:
- Replication lag (all standbys)
- WAL generation rate
- Replication throughput
- Standby status
- Sync/async mode

#### 5. Security Dashboard

**Panels**:
- Failed login attempts
- Threat detection events
- Audit log activity
- Permission violations

### Example Dashboard JSON

```json
{
  "dashboard": {
    "title": "RustyDB Overview",
    "panels": [
      {
        "id": 1,
        "title": "Transactions Per Second",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(rustydb_transactions_total[1m])",
            "legendFormat": "TPS"
          }
        ]
      },
      {
        "id": 2,
        "title": "Cache Hit Ratio",
        "type": "singlestat",
        "targets": [
          {
            "expr": "rustydb_cache_hit_ratio * 100"
          }
        ],
        "format": "percent"
      }
    ]
  }
}
```

---

## Alerting Strategy

### Alert Rules

**Prometheus Alert Rules** (`alerts.yml`):
```yaml
groups:
  - name: rustydb_alerts
    interval: 30s
    rules:
      # Critical Alerts
      - alert: RustyDBDown
        expr: up{job="rustydb"} == 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "RustyDB instance {{ $labels.instance }} is down"
          description: "Database has been unreachable for 1 minute"

      - alert: HighCPUUsage
        expr: rustydb_cpu_usage_percent > 90
        for: 5m
        labels:
          severity: critical
        annotations:
          summary: "High CPU usage on {{ $labels.instance }}"
          description: "CPU usage is {{ $value }}%"

      - alert: LowCacheHitRatio
        expr: rustydb_cache_hit_ratio < 0.80
        for: 10m
        labels:
          severity: critical
        annotations:
          summary: "Low cache hit ratio on {{ $labels.instance }}"
          description: "Cache hit ratio is {{ $value }}%"

      # Warning Alerts
      - alert: HighMemoryUsage
        expr: rustydb_memory_usage_percent > 85
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High memory usage on {{ $labels.instance }}"
          description: "Memory usage is {{ $value }}%"

      - alert: HighDiskUsage
        expr: rustydb_disk_usage_percent > 80
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High disk usage on {{ $labels.instance }}"
          description: "Disk usage is {{ $value }}%"

      - alert: ReplicationLag
        expr: rustydb_replication_lag_seconds > 30
        for: 2m
        labels:
          severity: warning
        annotations:
          summary: "High replication lag on {{ $labels.standby }}"
          description: "Replication lag is {{ $value }} seconds"

      - alert: HighConnectionUsage
        expr: (rustydb_active_connections / rustydb_max_connections) * 100 > 90
        for: 5m
        labels:
          severity: warning
        annotations:
          summary: "High connection usage on {{ $labels.instance }}"
          description: "{{ $value }}% of max connections in use"

      - alert: HighQueryLatency
        expr: rate(rustydb_query_duration_seconds_sum[5m]) / rate(rustydb_query_duration_seconds_count[5m]) > 1.0
        for: 10m
        labels:
          severity: warning
        annotations:
          summary: "High query latency on {{ $labels.instance }}"
          description: "Average query latency is {{ $value }}s"

      # Security Alerts
      - alert: HighFailedLogins
        expr: rate(rustydb_failed_logins_total[5m]) > 10
        for: 1m
        labels:
          severity: warning
        annotations:
          summary: "High failed login rate on {{ $labels.instance }}"
          description: "{{ $value }} failed logins per second"

      - alert: SecurityThreatDetected
        expr: increase(rustydb_threats_detected_total[1m]) > 0
        for: 1m
        labels:
          severity: critical
        annotations:
          summary: "Security threat detected on {{ $labels.instance }}"
          description: "{{ $value }} threats detected in last minute"
```

### AlertManager Configuration

```yaml
global:
  resolve_timeout: 5m
  smtp_smarthost: 'smtp.company.com:587'
  smtp_from: 'alerts@company.com'
  smtp_auth_username: 'alerts@company.com'
  smtp_auth_password: 'password'

route:
  group_by: ['alertname', 'instance']
  group_wait: 10s
  group_interval: 10s
  repeat_interval: 1h
  receiver: 'team-dba'

  routes:
    - match:
        severity: critical
      receiver: 'team-dba-pagerduty'
      continue: true

    - match:
        severity: warning
      receiver: 'team-dba-email'

receivers:
  - name: 'team-dba-email'
    email_configs:
      - to: 'dba-team@company.com'
        headers:
          Subject: 'RustyDB Alert: {{ .GroupLabels.alertname }}'

  - name: 'team-dba-pagerduty'
    pagerduty_configs:
      - service_key: 'YOUR_PAGERDUTY_KEY'

  - name: 'team-dba-slack'
    slack_configs:
      - api_url: 'https://hooks.slack.com/services/YOUR/SLACK/WEBHOOK'
        channel: '#database-alerts'
        text: '{{ range .Alerts }}{{ .Annotations.description }}{{ end }}'
```

---

## Log Monitoring

### Log Aggregation with ELK Stack

**1. Install Filebeat**:
```bash
curl -L -O https://artifacts.elastic.co/downloads/beats/filebeat/filebeat-8.11.0-amd64.deb
sudo dpkg -i filebeat-8.11.0-amd64.deb
```

**2. Configure Filebeat** (`/etc/filebeat/filebeat.yml`):
```yaml
filebeat.inputs:
  - type: log
    enabled: true
    paths:
      - /var/lib/rustydb/instances/*/logs/*.log
    fields:
      application: rustydb
      environment: production
    json.keys_under_root: true
    json.add_error_key: true

output.elasticsearch:
  hosts: ["elasticsearch.company.com:9200"]
  index: "rustydb-logs-%{+yyyy.MM.dd}"

setup.kibana:
  host: "kibana.company.com:5601"

setup.ilm.enabled: false
setup.template.name: "rustydb"
setup.template.pattern: "rustydb-*"
```

**3. Start Filebeat**:
```bash
sudo systemctl enable filebeat
sudo systemctl start filebeat
```

### Log Analysis Queries

**Kibana/Elasticsearch Queries**:

```json
# Error rate over time
{
  "query": {
    "bool": {
      "must": [
        { "match": { "level": "error" } },
        { "range": { "@timestamp": { "gte": "now-1h" } } }
      ]
    }
  }
}

# Failed login attempts
{
  "query": {
    "bool": {
      "must": [
        { "match": { "event": "failed_login" } },
        { "range": { "@timestamp": { "gte": "now-24h" } } }
      ]
    }
  }
}

# Slow queries
{
  "query": {
    "bool": {
      "must": [
        { "match": { "event": "query_execution" } },
        { "range": { "duration_ms": { "gte": 1000 } } }
      ]
    }
  }
}
```

---

## Application Performance Monitoring

### APM Integration

**New Relic Integration**:
```toml
# conf/rustydb.toml
[apm]
enabled = true
provider = "newrelic"
license_key = "YOUR_LICENSE_KEY"
app_name = "RustyDB Production"
```

**Datadog Integration**:
```bash
# Install Datadog agent
DD_AGENT_MAJOR_VERSION=7 DD_API_KEY=YOUR_API_KEY DD_SITE="datadoghq.com" bash -c "$(curl -L https://s.datadoghq.com/scripts/install_script.sh)"

# Configure RustyDB integration
cat > /etc/datadog-agent/conf.d/rustydb.yaml << EOF
init_config:

instances:
  - prometheus_url: http://localhost:9100/metrics
    namespace: rustydb
    metrics:
      - '*'
    tags:
      - env:production
      - service:rustydb
EOF

sudo systemctl restart datadog-agent
```

---

## Capacity Monitoring

### Capacity Metrics

```sql
-- Database growth rate
SELECT
    metric_date,
    total_data_size_gb,
    total_data_size_gb - LAG(total_data_size_gb) OVER (ORDER BY metric_date) as daily_growth_gb
FROM v$capacity_metrics
WHERE metric_date >= NOW() - INTERVAL '30 days'
ORDER BY metric_date DESC;

-- Forecast capacity
SELECT
    resource_type,
    current_usage_percent,
    days_until_80_percent,
    days_until_90_percent,
    days_until_full
FROM v$capacity_forecast
WHERE days_until_90_percent < 90;
```

---

## Security Monitoring

### Security Event Monitoring

```sql
-- Failed login attempts
SELECT
    username,
    ip_address,
    COUNT(*) as attempts,
    MAX(timestamp) as last_attempt
FROM v$failed_logins
WHERE timestamp > NOW() - INTERVAL '1 hour'
GROUP BY username, ip_address
HAVING COUNT(*) > 5
ORDER BY attempts DESC;

-- Suspicious queries
SELECT
    username,
    query,
    threat_score,
    threat_category
FROM v$suspicious_queries
WHERE threat_score > 70
ORDER BY threat_score DESC;

-- Audit log review
SELECT
    timestamp,
    username,
    action,
    object_name,
    status
FROM v$audit_log
WHERE timestamp > NOW() - INTERVAL '24 hours'
AND status = 'DENIED'
ORDER BY timestamp DESC;
```

---

**Document Maintained By**: Enterprise Documentation Agent 4
**RustyDB Version**: 0.6.0
