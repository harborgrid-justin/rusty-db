// Self-service tenant provisioning with workflows and automation
// Implements template-based provisioning, tier configuration, and lifecycle management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::fmt;
use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;

// Provisioning error types
#[derive(Debug, Clone)]
pub enum ProvisioningError {
    TemplateNotFound(String),
    InvalidConfiguration(String),
    QuotaExceeded(String),
    ApprovalRequired(String),
    ProvisioningFailed(String),
    DeprovisioningFailed(String),
    WorkflowError(String),
}

impl fmt::Display for ProvisioningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProvisioningError::TemplateNotFound(name) => write!(f, "Template not found: {}", name),
            ProvisioningError::InvalidConfiguration(msg) => {
                write!(f, "Invalid configuration: {}", msg)
            }
            ProvisioningError::QuotaExceeded(msg) => write!(f, "Quota exceeded: {}", msg),
            ProvisioningError::ApprovalRequired(msg) => write!(f, "Approval required: {}", msg),
            ProvisioningError::ProvisioningFailed(msg) => write!(f, "Provisioning failed: {}", msg),
            ProvisioningError::DeprovisioningFailed(msg) => {
                write!(f, "Deprovisioning failed: {}", msg)
            }
            ProvisioningError::WorkflowError(msg) => write!(f, "Workflow error: {}", msg),
        }
    }
}

impl std::error::Error for ProvisioningError {}

pub type ProvisioningResult<T> = Result<T, ProvisioningError>;

// Provisioning template for different service tiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisioningTemplate {
    pub template_id: String,
    pub template_name: String,
    pub tier: ServiceTier,
    pub description: String,
    pub cpu_cores: f64,
    pub memory_mb: u64,
    pub storage_gb: u64,
    pub iops_limit: u32,
    pub network_mbps: u32,
    pub max_connections: u32,
    pub backup_enabled: bool,
    pub backup_retention_days: u32,
    pub high_availability: bool,
    pub encryption_at_rest: bool,
    pub encryption_in_transit: bool,
    pub monitoring_enabled: bool,
    pub default_schemas: Vec<String>,
    pub default_tablespaces: Vec<String>,
    pub initialization_scripts: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceTier {
    Free,
    Basic,
    Standard,
    Premium,
    Enterprise,
}

impl ProvisioningTemplate {
    pub fn free_tier() -> Self {
        Self {
            template_id: "tpl_free".to_string(),
            template_name: "Free Tier".to_string(),
            tier: ServiceTier::Free,
            description: "Free tier with basic features".to_string(),
            cpu_cores: 0.5,
            memory_mb: 512,
            storage_gb: 5,
            iops_limit: 100,
            network_mbps: 10,
            max_connections: 10,
            backup_enabled: false,
            backup_retention_days: 0,
            high_availability: false,
            encryption_at_rest: false,
            encryption_in_transit: false,
            monitoring_enabled: false,
            default_schemas: vec!["PUBLIC".to_string()],
            default_tablespaces: vec!["USERS".to_string()],
            initialization_scripts: vec![],
        }
    }

    pub fn basic_tier() -> Self {
        Self {
            template_id: "tpl_basic".to_string(),
            template_name: "Basic Tier".to_string(),
            tier: ServiceTier::Basic,
            description: "Basic tier for small workloads".to_string(),
            cpu_cores: 1.0,
            memory_mb: 2048,
            storage_gb: 20,
            iops_limit: 500,
            network_mbps: 50,
            max_connections: 25,
            backup_enabled: true,
            backup_retention_days: 7,
            high_availability: false,
            encryption_at_rest: false,
            encryption_in_transit: true,
            monitoring_enabled: true,
            default_schemas: vec!["PUBLIC".to_string()],
            default_tablespaces: vec!["USERS".to_string(), "TEMP".to_string()],
            initialization_scripts: vec!["init_basic.sql".to_string()],
        }
    }

    pub fn standard_tier() -> Self {
        Self {
            template_id: "tpl_standard".to_string(),
            template_name: "Standard Tier".to_string(),
            tier: ServiceTier::Standard,
            description: "Standard tier for production workloads".to_string(),
            cpu_cores: 2.0,
            memory_mb: 4096,
            storage_gb: 100,
            iops_limit: 2000,
            network_mbps: 100,
            max_connections: 100,
            backup_enabled: true,
            backup_retention_days: 14,
            high_availability: false,
            encryption_at_rest: true,
            encryption_in_transit: true,
            monitoring_enabled: true,
            default_schemas: vec!["PUBLIC".to_string(), "APP".to_string()],
            default_tablespaces: vec!["USERS".to_string(), "TEMP".to_string(), "INDEX".to_string()],
            initialization_scripts: vec!["init_standard.sql".to_string()],
        }
    }

    pub fn premium_tier() -> Self {
        Self {
            template_id: "tpl_premium".to_string(),
            template_name: "Premium Tier".to_string(),
            tier: ServiceTier::Premium,
            description: "Premium tier with high availability".to_string(),
            cpu_cores: 4.0,
            memory_mb: 8192,
            storage_gb: 250,
            iops_limit: 5000,
            network_mbps: 250,
            max_connections: 250,
            backup_enabled: true,
            backup_retention_days: 30,
            high_availability: true,
            encryption_at_rest: true,
            encryption_in_transit: true,
            monitoring_enabled: true,
            default_schemas: vec![
                "PUBLIC".to_string(),
                "APP".to_string(),
                "ANALYTICS".to_string(),
            ],
            default_tablespaces: vec![
                "USERS".to_string(),
                "TEMP".to_string(),
                "INDEX".to_string(),
                "LOB".to_string(),
            ],
            initialization_scripts: vec![
                "init_premium.sql".to_string(),
                "setup_ha.sql".to_string(),
            ],
        }
    }

    pub fn enterprise_tier() -> Self {
        Self {
            template_id: "tpl_enterprise".to_string(),
            template_name: "Enterprise Tier".to_string(),
            tier: ServiceTier::Enterprise,
            description: "Enterprise tier with all features".to_string(),
            cpu_cores: 8.0,
            memory_mb: 16384,
            storage_gb: 1000,
            iops_limit: 10000,
            network_mbps: 1000,
            max_connections: 500,
            backup_enabled: true,
            backup_retention_days: 90,
            high_availability: true,
            encryption_at_rest: true,
            encryption_in_transit: true,
            monitoring_enabled: true,
            default_schemas: vec![
                "PUBLIC".to_string(),
                "APP".to_string(),
                "ANALYTICS".to_string(),
                "ARCHIVE".to_string(),
            ],
            default_tablespaces: vec![
                "USERS".to_string(),
                "TEMP".to_string(),
                "INDEX".to_string(),
                "LOB".to_string(),
                "ARCHIVE".to_string(),
            ],
            initialization_scripts: vec![
                "init_enterprise.sql".to_string(),
                "setup_ha.sql".to_string(),
                "setup_monitoring.sql".to_string(),
            ],
        }
    }
}

// Provisioning request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisioningRequest {
    pub request_id: String,
    pub requester: String,
    pub tenant_name: String,
    pub template_id: String,
    pub custom_config: HashMap<String, String>,
    pub justification: String,
    pub cost_center: String,
    pub requested_at: SystemTime,
    pub status: RequestStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RequestStatus {
    Pending,
    UnderReview,
    Approved,
    Rejected,
    Provisioning,
    Completed,
    Failed,
}

// Provisioning workflow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvisioningWorkflow {
    pub workflow_id: String,
    pub request_id: String,
    pub steps: Vec<WorkflowStep>,
    pub current_step: usize,
    pub status: WorkflowStatus,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowStep {
    pub step_id: u32,
    pub step_name: String,
    pub step_type: StepType,
    pub status: StepStatus,
    pub started_at: Option<SystemTime>,
    pub completed_at: Option<SystemTime>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepType {
    ValidateRequest,
    CheckQuota,
    RequestApproval,
    AllocateResources,
    CreateDatabase,
    InitializeSchemas,
    ConfigureSecurity,
    SetupMonitoring,
    SetupBackup,
    RunInitScripts,
    ValidateDeployment,
    NotifyUser,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Skipped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkflowStatus {
    NotStarted,
    InProgress,
    Completed,
    Failed,
    Cancelled,
}

// Deprovisioning policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprovisioningPolicy {
    pub retention_days: u32,
    pub backup_before_delete: bool,
    pub archive_data: bool,
    pub archive_location: String,
    pub notify_users: bool,
    pub grace_period_days: u32,
}

impl Default for DeprovisioningPolicy {
    fn default() -> Self {
        Self {
            retention_days: 30,
            backup_before_delete: true,
            archive_data: true,
            archive_location: "/archives".to_string(),
            notify_users: true,
            grace_period_days: 7,
        }
    }
}

// Deprovisioning request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeprovisioningRequest {
    pub request_id: String,
    pub tenant_id: String,
    pub requester: String,
    pub reason: String,
    pub policy: DeprovisioningPolicy,
    pub requested_at: SystemTime,
    pub scheduled_at: SystemTime,
    pub status: RequestStatus,
}

// Provisioning service
pub struct ProvisioningService {
    templates: Arc<RwLock<HashMap<String, ProvisioningTemplate>>>,
    requests: Arc<RwLock<HashMap<String, ProvisioningRequest>>>,
    workflows: Arc<RwLock<HashMap<String, ProvisioningWorkflow>>>,
    deprovisioning_requests: Arc<RwLock<HashMap<String, DeprovisioningRequest>>>,
    approval_queue: Arc<RwLock<VecDeque<String>>>,
    quota_limits: Arc<RwLock<QuotaLimits>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaLimits {
    pub max_tenants_per_user: u32,
    pub max_total_cpu: f64,
    pub max_total_memory_mb: u64,
    pub max_total_storage_gb: u64,
    pub requires_approval_above_tier: ServiceTier,
}

impl Default for QuotaLimits {
    fn default() -> Self {
        Self {
            max_tenants_per_user: 10,
            max_total_cpu: 100.0,
            max_total_memory_mb: 102400,
            max_total_storage_gb: 10000,
            requires_approval_above_tier: ServiceTier::Standard,
        }
    }
}

impl ProvisioningService {
    pub fn new() -> Self {
        let mut templates = HashMap::new();

        // Register default templates
        let tpl_free = ProvisioningTemplate::free_tier();
        let tpl_basic = ProvisioningTemplate::basic_tier();
        let tpl_standard = ProvisioningTemplate::standard_tier();
        let tpl_premium = ProvisioningTemplate::premium_tier();
        let tpl_enterprise = ProvisioningTemplate::enterprise_tier();

        templates.insert(tpl_free.template_id.clone(), tpl_free);
        templates.insert(tpl_basic.template_id.clone(), tpl_basic);
        templates.insert(tpl_standard.template_id.clone(), tpl_standard);
        templates.insert(tpl_premium.template_id.clone(), tpl_premium);
        templates.insert(tpl_enterprise.template_id.clone(), tpl_enterprise);

        Self {
            templates: Arc::new(RwLock::new(templates)),
            requests: Arc::new(RwLock::new(HashMap::new())),
            workflows: Arc::new(RwLock::new(HashMap::new())),
            deprovisioning_requests: Arc::new(RwLock::new(HashMap::new())),
            approval_queue: Arc::new(RwLock::new(VecDeque::new())),
            quota_limits: Arc::new(RwLock::new(QuotaLimits::default())),
        }
    }

    // Register a custom template
    pub async fn register_template(&self, template: ProvisioningTemplate) {
        let mut templates = self.templates.write().await;
        templates.insert(template.template_id.clone(), template);
    }

    // Get template by ID
    pub async fn get_template(&self, template_id: &str) -> Option<ProvisioningTemplate> {
        let templates = self.templates.read().await;
        templates.get(template_id).cloned()
    }

    // List all templates
    pub async fn list_templates(&self) -> Vec<ProvisioningTemplate> {
        let templates = self.templates.read().await;
        templates.values().cloned().collect()
    }

    // Submit provisioning request
    pub async fn submit_request(
        &self,
        requester: String,
        tenant_name: String,
        template_id: String,
        custom_config: HashMap<String, String>,
        justification: String,
        cost_center: String,
    ) -> ProvisioningResult<String> {
        // Validate template exists
        let templates = self.templates.read().await;
        let template = templates
            .get(&template_id)
            .ok_or_else(|| ProvisioningError::TemplateNotFound(template_id.clone()))?;

        let tier = template.tier;
        drop(templates);

        // Check quota limits
        self.check_quota_limits(&requester, &template_id).await?;

        let request_id = uuid::Uuid::new_v4().to_string();

        let request = ProvisioningRequest {
            request_id: request_id.clone(),
            requester: requester.clone(),
            tenant_name,
            template_id,
            custom_config,
            justification,
            cost_center,
            requested_at: SystemTime::now(),
            status: RequestStatus::Pending,
        };

        let mut requests = self.requests.write().await;
        requests.insert(request_id.clone(), request);
        drop(requests);

        // Check if approval is required
        let quota_limits = self.quota_limits.read().await;
        if tier as u32 > quota_limits.requires_approval_above_tier as u32 {
            drop(quota_limits);
            self.request_approval(&request_id).await?;
        } else {
            drop(quota_limits);
            // Auto-approve and start provisioning
            self.approve_request(&request_id).await?;
        }

        Ok(request_id)
    }

    async fn check_quota_limits(
        &self,
        _requester: &str,
        _template_id: &str,
    ) -> ProvisioningResult<()> {
        // Simplified quota check
        // In a real implementation, would check user's current usage
        Ok(())
    }

    async fn request_approval(&self, request_id: &str) -> ProvisioningResult<()> {
        let mut requests = self.requests.write().await;
        if let Some(request) = requests.get_mut(request_id) {
            request.status = RequestStatus::UnderReview;
        }
        drop(requests);

        let mut approval_queue = self.approval_queue.write().await;
        approval_queue.push_back(request_id.to_string());

        Err(ProvisioningError::ApprovalRequired(
            "Request requires approval".to_string(),
        ))
    }

    // Approve provisioning request
    pub async fn approve_request(&self, request_id: &str) -> ProvisioningResult<()> {
        let mut requests = self.requests.write().await;
        let request = requests
            .get_mut(request_id)
            .ok_or_else(|| ProvisioningError::WorkflowError("Request not found".to_string()))?;

        request.status = RequestStatus::Approved;
        drop(requests);

        // Create and start workflow
        self.create_workflow(request_id).await?;

        Ok(())
    }

    // Reject provisioning request
    pub async fn reject_request(&self, request_id: &str, reason: String) -> ProvisioningResult<()> {
        let mut requests = self.requests.write().await;
        if let Some(request) = requests.get_mut(request_id) {
            request.status = RequestStatus::Rejected;
        }

        println!("Request {} rejected: {}", request_id, reason);

        Ok(())
    }

    async fn create_workflow(&self, request_id: &str) -> ProvisioningResult<()> {
        let workflow_id = uuid::Uuid::new_v4().to_string();

        let steps = vec![
            WorkflowStep {
                step_id: 1,
                step_name: "Validate Request".to_string(),
                step_type: StepType::ValidateRequest,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
            },
            WorkflowStep {
                step_id: 2,
                step_name: "Check Quota".to_string(),
                step_type: StepType::CheckQuota,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
            },
            WorkflowStep {
                step_id: 3,
                step_name: "Allocate Resources".to_string(),
                step_type: StepType::AllocateResources,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
            },
            WorkflowStep {
                step_id: 4,
                step_name: "Create Database".to_string(),
                step_type: StepType::CreateDatabase,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
            },
            WorkflowStep {
                step_id: 5,
                step_name: "Initialize Schemas".to_string(),
                step_type: StepType::InitializeSchemas,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
            },
            WorkflowStep {
                step_id: 6,
                step_name: "Configure Security".to_string(),
                step_type: StepType::ConfigureSecurity,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
            },
            WorkflowStep {
                step_id: 7,
                step_name: "Setup Monitoring".to_string(),
                step_type: StepType::SetupMonitoring,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
            },
            WorkflowStep {
                step_id: 8,
                step_name: "Setup Backup".to_string(),
                step_type: StepType::SetupBackup,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
            },
            WorkflowStep {
                step_id: 9,
                step_name: "Run Initialization Scripts".to_string(),
                step_type: StepType::RunInitScripts,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
            },
            WorkflowStep {
                step_id: 10,
                step_name: "Validate Deployment".to_string(),
                step_type: StepType::ValidateDeployment,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
            },
            WorkflowStep {
                step_id: 11,
                step_name: "Notify User".to_string(),
                step_type: StepType::NotifyUser,
                status: StepStatus::Pending,
                started_at: None,
                completed_at: None,
                output: None,
            },
        ];

        let workflow = ProvisioningWorkflow {
            workflow_id: workflow_id.clone(),
            request_id: request_id.to_string(),
            steps,
            current_step: 0,
            status: WorkflowStatus::NotStarted,
            started_at: None,
            completed_at: None,
            error_message: None,
        };

        let mut workflows = self.workflows.write().await;
        workflows.insert(workflow_id.clone(), workflow);
        drop(workflows);

        // Execute workflow asynchronously
        let service_clone = self.clone();
        let workflow_id_clone = workflow_id.clone();
        tokio::spawn(async move {
            let _ = service_clone.execute_workflow(&workflow_id_clone).await;
        });

        Ok(())
    }

    async fn execute_workflow(&self, workflow_id: &str) -> ProvisioningResult<()> {
        // Update workflow status
        let mut workflows = self.workflows.write().await;
        let workflow = workflows
            .get_mut(workflow_id)
            .ok_or_else(|| ProvisioningError::WorkflowError("Workflow not found".to_string()))?;

        workflow.status = WorkflowStatus::InProgress;
        workflow.started_at = Some(SystemTime::now());
        drop(workflows);

        // Execute each step
        loop {
            let mut workflows = self.workflows.write().await;
            let workflow = workflows.get_mut(workflow_id).unwrap();

            if workflow.current_step >= workflow.steps.len() {
                workflow.status = WorkflowStatus::Completed;
                workflow.completed_at = Some(SystemTime::now());
                break;
            }

            let step = &mut workflow.steps[workflow.current_step];
            step.status = StepStatus::Running;
            step.started_at = Some(SystemTime::now());

            drop(workflows);

            // Simulate step execution
            tokio::time::sleep(Duration::from_millis(100)).await;

            let mut workflows = self.workflows.write().await;
            let workflow = workflows.get_mut(workflow_id).unwrap();
            let step = &mut workflow.steps[workflow.current_step];

            step.status = StepStatus::Completed;
            step.completed_at = Some(SystemTime::now());
            step.output = Some(format!("Step {} completed", step.step_id));

            workflow.current_step += 1;
        }

        // Update request status
        let workflows = self.workflows.read().await;
        let workflow = workflows.get(workflow_id).unwrap();
        let request_id = workflow.request_id.clone();
        drop(workflows);

        let mut requests = self.requests.write().await;
        if let Some(request) = requests.get_mut(&request_id) {
            request.status = RequestStatus::Completed;
        }

        Ok(())
    }

    // Get workflow status
    pub async fn get_workflow_status(&self, workflow_id: &str) -> Option<ProvisioningWorkflow> {
        let workflows = self.workflows.read().await;
        workflows.get(workflow_id).cloned()
    }

    // Submit deprovisioning request
    pub async fn submit_deprovisioning_request(
        &self,
        tenant_id: String,
        requester: String,
        reason: String,
        policy: Option<DeprovisioningPolicy>,
    ) -> ProvisioningResult<String> {
        let request_id = uuid::Uuid::new_v4().to_string();
        let policy = policy.unwrap_or_default();

        let scheduled_at =
            SystemTime::now() + Duration::from_secs(policy.grace_period_days as u64 * 86400);

        let request = DeprovisioningRequest {
            request_id: request_id.clone(),
            tenant_id,
            requester,
            reason,
            policy,
            requested_at: SystemTime::now(),
            scheduled_at,
            status: RequestStatus::Pending,
        };

        let mut requests = self.deprovisioning_requests.write().await;
        requests.insert(request_id.clone(), request);

        Ok(request_id)
    }

    // Execute deprovisioning
    pub async fn execute_deprovisioning(&self, request_id: &str) -> ProvisioningResult<()> {
        let mut requests = self.deprovisioning_requests.write().await;
        let request = requests.get_mut(request_id).ok_or_else(|| {
            ProvisioningError::DeprovisioningFailed("Request not found".to_string())
        })?;

        request.status = RequestStatus::Provisioning;

        // Simulate deprovisioning steps
        if request.policy.backup_before_delete {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        if request.policy.archive_data {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        request.status = RequestStatus::Completed;

        Ok(())
    }

    // List pending approval requests
    pub async fn list_pending_approvals(&self) -> Vec<String> {
        let approval_queue = self.approval_queue.read().await;
        approval_queue.iter().cloned().collect()
    }

    fn clone(&self) -> Self {
        Self {
            templates: Arc::clone(&self.templates),
            requests: Arc::clone(&self.requests),
            workflows: Arc::clone(&self.workflows),
            deprovisioning_requests: Arc::clone(&self.deprovisioning_requests),
            approval_queue: Arc::clone(&self.approval_queue),
            quota_limits: Arc::clone(&self.quota_limits),
        }
    }
}

impl Default for ProvisioningService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[tokio::test]
    async fn test_template_registration() {
        let service = ProvisioningService::new();

        let templates = service.list_templates().await;
        assert!(templates.len() >= 5);

        let template = service.get_template("tpl_standard").await;
        assert!(template.is_some());
    }

    #[tokio::test]
    async fn test_provisioning_request() {
        let service = ProvisioningService::new();

        let result = service
            .submit_request(
                "user@example.com".to_string(),
                "test-tenant".to_string(),
                "tpl_basic".to_string(),
                HashMap::new(),
                "Testing".to_string(),
                "CC001".to_string(),
            )
            .await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_workflow_execution() {
        let service = ProvisioningService::new();

        let request_id = service
            .submit_request(
                "user@example.com".to_string(),
                "test-tenant".to_string(),
                "tpl_free".to_string(),
                HashMap::new(),
                "Testing".to_string(),
                "CC001".to_string(),
            )
            .await
            .unwrap();

        // Wait for workflow to complete
        tokio::time::sleep(Duration::from_secs(2)).await.await;

        // Verify request is completed
        let requests = service.requests.read().await;
        let request = requests.get(&request_id).unwrap();
        assert_eq!(request.status, RequestStatus::Completed);
    }

    #[tokio::test]
    async fn test_deprovisioning_request() {
        let service = ProvisioningService::new();

        let result = service
            .submit_deprovisioning_request(
                "tenant-123".to_string(),
                "admin@example.com".to_string(),
                "No longer needed".to_string(),
                None,
            )
            .await;

        assert!(result.is_ok());
    }
}
