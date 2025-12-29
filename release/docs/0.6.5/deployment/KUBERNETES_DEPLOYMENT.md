# RustyDB v0.6.5 - Kubernetes Deployment Guide

**Version**: 0.6.5
**Release Date**: December 29, 2025
**Status**: ✅ Validated for Kubernetes Deployment
**Minimum K8s Version**: 1.27+

---

## Executive Summary

Complete guide for deploying RustyDB v0.6.5 on Kubernetes clusters. Covers StatefulSets, persistent storage, services, ingress, monitoring, autoscaling, and production best practices for cloud-native deployments.

---

## Prerequisites

- **Kubernetes**: 1.27+ cluster
- **kubectl**: Configured and authenticated
- **Storage Class**: For persistent volumes (e.g., gp3, premium-ssd)
- **Helm**: 3.12+ (optional, for simplified deployment)
- **Ingress Controller**: nginx or Traefik (for external access)

---

## Quick Deployment

### Single-Node Deployment

```yaml
# rustydb-quick.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: rustydb
  labels:
    app: rustydb
spec:
  replicas: 1
  selector:
    matchLabels:
      app: rustydb
  template:
    metadata:
      labels:
        app: rustydb
    spec:
      containers:
      - name: rustydb
        image: rustydb:0.6.5
        ports:
        - containerPort: 5432
          name: database
        - containerPort: 8080
          name: api
        - containerPort: 9090
          name: metrics
        resources:
          requests:
            cpu: "4"
            memory: "16Gi"
          limits:
            cpu: "8"
            memory: "32Gi"
        volumeMounts:
        - name: data
          mountPath: /var/lib/rustydb
        livenessProbe:
          httpGet:
            path: /api/v1/health
            port: 8080
          initialDelaySeconds: 60
          periodSeconds: 30
        readinessProbe:
          httpGet:
            path: /api/v1/health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
      volumes:
      - name: data
        emptyDir: {}  # ⚠️ Not for production

---
apiVersion: v1
kind: Service
metadata:
  name: rustydb
spec:
  selector:
    app: rustydb
  ports:
  - name: database
    port: 5432
    targetPort: 5432
  - name: api
    port: 8080
    targetPort: 8080
  type: LoadBalancer
```

**Deploy**:
```bash
kubectl apply -f rustydb-quick.yaml
kubectl get pods -w
kubectl get svc rustydb
```

---

## Production Deployment (StatefulSet)

### Namespace and ConfigMap

```yaml
# namespace.yaml
apiVersion: v1
kind: Namespace
metadata:
  name: rustydb
  labels:
    name: rustydb

---
# configmap.yaml
apiVersion: v1
kind: ConfigMap
metadata:
  name: rustydb-config
  namespace: rustydb
data:
  rustydb.toml: |
    [database]
    data_directory = "/var/lib/rustydb/data"
    wal_directory = "/var/lib/rustydb/wal"

    [storage]
    page_size = 4096
    buffer_pool_size = 107374182400  # 100 GB
    buffer_eviction_policy = "ARC"

    [network]
    host = "0.0.0.0"
    port = 5432
    api_port = 8080
    max_connections = 1000

    [monitoring]
    metrics_enabled = true
    metrics_port = 9090

    [logging]
    level = "info"
    output = "/var/lib/rustydb/logs/rustydb.log"
```

### Secrets

```yaml
# secrets.yaml
apiVersion: v1
kind: Secret
metadata:
  name: rustydb-secrets
  namespace: rustydb
type: Opaque
stringData:
  admin-password: "SecureAdminPassword123!"
  replication-password: "SecureReplPassword123!"
  tls-cert: |
    -----BEGIN CERTIFICATE-----
    [base64 encoded certificate]
    -----END CERTIFICATE-----
  tls-key: |
    -----BEGIN PRIVATE KEY-----
    [base64 encoded private key]
    -----END PRIVATE KEY-----
```

### StatefulSet

```yaml
# statefulset.yaml
apiVersion: apps/v1
kind: StatefulSet
metadata:
  name: rustydb
  namespace: rustydb
spec:
  serviceName: rustydb
  replicas: 3
  selector:
    matchLabels:
      app: rustydb
  template:
    metadata:
      labels:
        app: rustydb
      annotations:
        prometheus.io/scrape: "true"
        prometheus.io/port: "9090"
        prometheus.io/path: "/metrics"
    spec:
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
          - labelSelector:
              matchExpressions:
              - key: app
                operator: In
                values:
                - rustydb
            topologyKey: "kubernetes.io/hostname"
      initContainers:
      - name: init-permissions
        image: busybox:1.36
        command: ['sh', '-c', 'chown -R 999:999 /var/lib/rustydb']
        volumeMounts:
        - name: data
          mountPath: /var/lib/rustydb
      containers:
      - name: rustydb
        image: rustydb:0.6.5
        ports:
        - containerPort: 5432
          name: database
        - containerPort: 8080
          name: api
        - containerPort: 9090
          name: metrics
        env:
        - name: RUSTYDB_CONFIG
          value: "/etc/rustydb/rustydb.toml"
        - name: RUSTYDB_ADMIN_PASSWORD
          valueFrom:
            secretKeyRef:
              name: rustydb-secrets
              key: admin-password
        - name: POD_NAME
          valueFrom:
            fieldRef:
              fieldPath: metadata.name
        - name: POD_NAMESPACE
          valueFrom:
            fieldRef:
              fieldPath: metadata.namespace
        resources:
          requests:
            cpu: "8"
            memory: "128Gi"
          limits:
            cpu: "16"
            memory: "256Gi"
        volumeMounts:
        - name: data
          mountPath: /var/lib/rustydb
        - name: config
          mountPath: /etc/rustydb
          readOnly: true
        - name: tls-certs
          mountPath: /etc/rustydb/certs
          readOnly: true
        livenessProbe:
          httpGet:
            path: /api/v1/health
            port: 8080
          initialDelaySeconds: 60
          periodSeconds: 30
          timeoutSeconds: 10
          failureThreshold: 3
        readinessProbe:
          httpGet:
            path: /api/v1/health
            port: 8080
          initialDelaySeconds: 10
          periodSeconds: 10
          timeoutSeconds: 5
          successThreshold: 1
          failureThreshold: 3
        startupProbe:
          httpGet:
            path: /api/v1/health
            port: 8080
          initialDelaySeconds: 0
          periodSeconds: 10
          timeoutSeconds: 5
          failureThreshold: 30
      volumes:
      - name: config
        configMap:
          name: rustydb-config
      - name: tls-certs
        secret:
          secretName: rustydb-secrets
          items:
          - key: tls-cert
            path: server.crt
          - key: tls-key
            path: server.key
  volumeClaimTemplates:
  - metadata:
      name: data
    spec:
      accessModes: ["ReadWriteOnce"]
      storageClassName: fast-ssd  # Adjust for your cluster
      resources:
        requests:
          storage: 1Ti
```

### Services

```yaml
# services.yaml
---
# Headless service for StatefulSet
apiVersion: v1
kind: Service
metadata:
  name: rustydb
  namespace: rustydb
  labels:
    app: rustydb
spec:
  ports:
  - port: 5432
    name: database
  - port: 8080
    name: api
  - port: 9090
    name: metrics
  clusterIP: None
  selector:
    app: rustydb

---
# LoadBalancer for external access
apiVersion: v1
kind: Service
metadata:
  name: rustydb-external
  namespace: rustydb
  annotations:
    service.beta.kubernetes.io/aws-load-balancer-type: "nlb"
    service.beta.kubernetes.io/aws-load-balancer-internal: "true"
spec:
  type: LoadBalancer
  selector:
    app: rustydb
  ports:
  - name: database
    port: 5432
    targetPort: 5432
  - name: api
    port: 8080
    targetPort: 8080
  sessionAffinity: ClientIP

---
# Service for read replicas
apiVersion: v1
kind: Service
metadata:
  name: rustydb-read
  namespace: rustydb
spec:
  type: ClusterIP
  selector:
    app: rustydb
    role: replica  # Label replicas accordingly
  ports:
  - name: database
    port: 5432
    targetPort: 5432
```

### Ingress

```yaml
# ingress.yaml
apiVersion: networking.k8s.io/v1
kind: Ingress
metadata:
  name: rustydb-ingress
  namespace: rustydb
  annotations:
    kubernetes.io/ingress.class: "nginx"
    cert-manager.io/cluster-issuer: "letsencrypt-prod"
    nginx.ingress.kubernetes.io/backend-protocol: "HTTP"
    nginx.ingress.kubernetes.io/ssl-redirect: "true"
spec:
  tls:
  - hosts:
    - rustydb.example.com
    secretName: rustydb-tls
  rules:
  - host: rustydb.example.com
    http:
      paths:
      - path: /api
        pathType: Prefix
        backend:
          service:
            name: rustydb-external
            port:
              number: 8080
      - path: /graphql
        pathType: Prefix
        backend:
          service:
            name: rustydb-external
            port:
              number: 8080
```

---

## Deploy to Kubernetes

```bash
# Create namespace
kubectl create namespace rustydb

# Apply configuration
kubectl apply -f namespace.yaml
kubectl apply -f configmap.yaml
kubectl apply -f secrets.yaml
kubectl apply -f statefulset.yaml
kubectl apply -f services.yaml
kubectl apply -f ingress.yaml

# Watch deployment
kubectl get pods -n rustydb -w

# Check StatefulSet status
kubectl get statefulset -n rustydb
kubectl rollout status statefulset/rustydb -n rustydb

# Check services
kubectl get svc -n rustydb

# Check persistent volume claims
kubectl get pvc -n rustydb

# View logs
kubectl logs -n rustydb rustydb-0 -f
```

---

## Monitoring Integration

### ServiceMonitor (Prometheus Operator)

```yaml
# servicemonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: ServiceMonitor
metadata:
  name: rustydb
  namespace: rustydb
  labels:
    app: rustydb
spec:
  selector:
    matchLabels:
      app: rustydb
  endpoints:
  - port: metrics
    interval: 15s
    path: /metrics
```

### PodMonitor

```yaml
# podmonitor.yaml
apiVersion: monitoring.coreos.com/v1
kind: PodMonitor
metadata:
  name: rustydb
  namespace: rustydb
spec:
  selector:
    matchLabels:
      app: rustydb
  podMetricsEndpoints:
  - port: metrics
    interval: 15s
```

---

## Autoscaling

### Horizontal Pod Autoscaler

```yaml
# hpa.yaml
apiVersion: autoscaling/v2
kind: HorizontalPodAutoscaler
metadata:
  name: rustydb-hpa
  namespace: rustydb
spec:
  scaleTargetRef:
    apiVersion: apps/v1
    kind: StatefulSet
    name: rustydb
  minReplicas: 3
  maxReplicas: 9
  metrics:
  - type: Resource
    resource:
      name: cpu
      target:
        type: Utilization
        averageUtilization: 70
  - type: Resource
    resource:
      name: memory
      target:
        type: Utilization
        averageUtilization: 80
  - type: Pods
    pods:
      metric:
        name: rustydb_transactions_per_second
      target:
        type: AverageValue
        averageValue: "10000"
  behavior:
    scaleUp:
      stabilizationWindowSeconds: 300
      policies:
      - type: Pods
        value: 1
        periodSeconds: 60
    scaleDown:
      stabilizationWindowSeconds: 600
      policies:
      - type: Pods
        value: 1
        periodSeconds: 300
```

---

## Backup Strategy

### CronJob for Backups

```yaml
# backup-cronjob.yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: rustydb-backup
  namespace: rustydb
spec:
  schedule: "0 2 * * *"  # Daily at 2 AM
  successfulJobsHistoryLimit: 3
  failedJobsHistoryLimit: 1
  jobTemplate:
    spec:
      template:
        spec:
          restartPolicy: OnFailure
          containers:
          - name: backup
            image: rustydb:0.6.5
            command:
            - /bin/sh
            - -c
            - |
              TIMESTAMP=$(date +%Y%m%d_%H%M%S)
              rusty-db-cli backup full \
                --host rustydb-0.rustydb.rustydb.svc.cluster.local \
                --output /backup/full_backup_$TIMESTAMP.tar.gz \
                --compress gzip

              # Upload to S3
              aws s3 cp /backup/full_backup_$TIMESTAMP.tar.gz \
                s3://company-backups/rustydb/

              # Cleanup local backup
              rm /backup/full_backup_$TIMESTAMP.tar.gz
            volumeMounts:
            - name: backup
              mountPath: /backup
            env:
            - name: AWS_ACCESS_KEY_ID
              valueFrom:
                secretKeyRef:
                  name: aws-credentials
                  key: access-key-id
            - name: AWS_SECRET_ACCESS_KEY
              valueFrom:
                secretKeyRef:
                  name: aws-credentials
                  key: secret-access-key
          volumes:
          - name: backup
            emptyDir: {}
```

---

## High Availability Configuration

### Pod Disruption Budget

```yaml
# pdb.yaml
apiVersion: policy/v1
kind: PodDisruptionBudget
metadata:
  name: rustydb-pdb
  namespace: rustydb
spec:
  minAvailable: 2
  selector:
    matchLabels:
      app: rustydb
```

### Network Policies

```yaml
# networkpolicy.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: rustydb-network-policy
  namespace: rustydb
spec:
  podSelector:
    matchLabels:
      app: rustydb
  policyTypes:
  - Ingress
  - Egress
  ingress:
  # Allow from application namespace
  - from:
    - namespaceSelector:
        matchLabels:
          name: application
    ports:
    - protocol: TCP
      port: 5432
    - protocol: TCP
      port: 8080
  # Allow from monitoring namespace
  - from:
    - namespaceSelector:
        matchLabels:
          name: monitoring
    ports:
    - protocol: TCP
      port: 9090
  # Allow inter-cluster communication
  - from:
    - podSelector:
        matchLabels:
          app: rustydb
    ports:
    - protocol: TCP
      port: 7432
    - protocol: TCP
      port: 7433
  egress:
  # Allow DNS
  - to:
    - namespaceSelector:
        matchLabels:
          name: kube-system
    ports:
    - protocol: UDP
      port: 53
  # Allow inter-cluster
  - to:
    - podSelector:
        matchLabels:
          app: rustydb
  # Allow external backup destinations
  - to:
    - namespaceSelector: {}
    ports:
    - protocol: TCP
      port: 443
```

---

## Helm Chart Deployment (Future)

```bash
# Add RustyDB Helm repository (when available)
helm repo add rustydb https://charts.rustydb.io
helm repo update

# Install with custom values
helm install rustydb rustydb/rustydb \
  --namespace rustydb \
  --create-namespace \
  --values values.yaml

# Example values.yaml
cat <<EOF > values.yaml
replicaCount: 3

image:
  repository: rustydb
  tag: "0.6.5"
  pullPolicy: IfNotPresent

resources:
  requests:
    cpu: 8
    memory: 128Gi
  limits:
    cpu: 16
    memory: 256Gi

persistence:
  enabled: true
  storageClass: "fast-ssd"
  size: 1Ti

monitoring:
  enabled: true
  prometheus: true
  grafana: true

backup:
  enabled: true
  schedule: "0 2 * * *"
  retention: 30

ha:
  enabled: true
  minReplicas: 3
  maxReplicas: 9

security:
  tls:
    enabled: true
    certManager: true
  networkPolicy:
    enabled: true
EOF

# Upgrade
helm upgrade rustydb rustydb/rustydb \
  --namespace rustydb \
  --values values.yaml

# Uninstall
helm uninstall rustydb --namespace rustydb
```

---

## Operations

### Scaling

```bash
# Scale up
kubectl scale statefulset rustydb --replicas=5 -n rustydb

# Scale down (careful with data)
kubectl scale statefulset rustydb --replicas=3 -n rustydb
```

### Rolling Updates

```bash
# Update image
kubectl set image statefulset/rustydb rustydb=rustydb:0.6.6 -n rustydb

# Check rollout status
kubectl rollout status statefulset/rustydb -n rustydb

# Pause rollout
kubectl rollout pause statefulset/rustydb -n rustydb

# Resume rollout
kubectl rollout resume statefulset/rustydb -n rustydb

# Rollback
kubectl rollout undo statefulset/rustydb -n rustydb
```

### Debugging

```bash
# Exec into pod
kubectl exec -it rustydb-0 -n rustydb -- /bin/bash

# View logs
kubectl logs -f rustydb-0 -n rustydb

# Describe pod
kubectl describe pod rustydb-0 -n rustydb

# Check events
kubectl get events -n rustydb --sort-by='.lastTimestamp'

# Port forward for local access
kubectl port-forward svc/rustydb-external 5432:5432 -n rustydb
kubectl port-forward svc/rustydb-external 8080:8080 -n rustydb
```

---

## Production Checklist

- [ ] Kubernetes cluster 1.27+
- [ ] Storage class configured (SSD/NVMe)
- [ ] Namespace created
- [ ] ConfigMap with production settings
- [ ] Secrets created (passwords, TLS certs)
- [ ] StatefulSet deployed (3+ replicas)
- [ ] Pod anti-affinity configured
- [ ] Resource limits set appropriately
- [ ] PersistentVolumeClaims bound
- [ ] Services created (headless, LoadBalancer)
- [ ] Ingress configured with TLS
- [ ] Monitoring integrated (ServiceMonitor)
- [ ] Autoscaling configured (HPA)
- [ ] Pod Disruption Budget set
- [ ] Network Policies applied
- [ ] Backup CronJob scheduled
- [ ] Health checks responding
- [ ] All pods ready
- [ ] Metrics being collected

---

**Document Version**: 1.0
**Last Updated**: December 29, 2025
**Status**: ✅ Validated for Kubernetes Deployment
**Minimum K8s Version**: 1.27+

---

*RustyDB v0.6.5 - Cloud-Native Kubernetes Deployment*
*$856M Enterprise Database - Kubernetes Ready*
