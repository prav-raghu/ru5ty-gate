mod api_response;
mod batch;
mod disposable_email_domains;
mod email_domain;
mod field_error;
mod permission;
mod rbac;
mod reporting;
mod role_name;
mod token_scope;
mod upload;
mod webhook;

pub use api_response::ApiResponse;
pub use batch::{
    BatchOperation, BatchOperationItem, BatchOperationOptions, BatchOperationResult,
    BatchOperationSummary, BatchOperationType,
};
pub use disposable_email_domains::DISPOSABLE_EMAIL_DOMAINS;
pub use email_domain::is_disposable_email;
pub use field_error::FieldError;
pub use permission::Permission;
pub use rbac::{get_permissions_for_role, role_has_permission};
pub use reporting::{
    ReportFilter, ReportFormat, ReportRequest, ReportResult, ReportStatus, ReportType,
    ScheduledReport,
};
pub use role_name::{ADMIN_TIER_ROLES, CUSTOMER_TIER_ROLES, RoleName};
pub use token_scope::TokenScope;
pub use upload::{
    DeleteFileOptions, FileAccessLevel, FileMetadata, ListFilesOptions, ListFilesResult,
    SignedUrlOptions, SignedUrlResult, StorageProvider, UploadOptions, UploadResult,
};
pub use webhook::{
    CreateWebhookSubscriptionDto, UpdateWebhookSubscriptionDto, WebhookDeliveryStatus,
    WebhookEventType, WebhookPayload,
};
