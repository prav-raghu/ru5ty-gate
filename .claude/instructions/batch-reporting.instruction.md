# Batch Operations & Reporting Documentation

## Overview

The batch operations and reporting system provides comprehensive tools for administrators to perform bulk actions and generate system-wide reports. These features are primarily implemented in the **admin-api** service.

## Implementation (Rust, admin-api)

| Concern | Location |
|---|---|
| Batch routes | `apps/backend/admin-api/src/routes/batch_route.rs` — all behind `Permission::BatchWrite` |
| Reporting routes | `apps/backend/admin-api/src/routes/reporting_route.rs` — views behind `Permission::ReportView`, exports behind `Permission::ReportExport` |
| Controllers | `controllers/batch_controller.rs`, `controllers/reporting_controller.rs` |
| Services | `services/batch_operation_service.rs`, `services/reporting_service.rs` |
| Request structs | `schemas/batch_schema.rs`, `schemas/report_schema.rs` |
| Shared result types | `common/types/src/batch.rs`, `common/types/src/reporting.rs` |
| Export formats | `ru5ty-gate-export` (CSV and Excel) |

The response wraps results in the standard envelope: the top-level flag is `isSuccessful` (false when any item failed), per-item results keep `success`. Examples below that show a top-level `success` key use the older shorthand; the real top-level key is `isSuccessful`. Roles `Super Admin` and `Moderator` hold `BatchWrite` and the report permissions; `Support` can view reports but cannot batch-write or export (see `rbac` agent).

Limits enforced by the request structs: bulk operations accept 1–500 items, custom batches 1–100 items. Every request body rejects unknown fields.

## Batch Operations

### Features
- **Transaction Support**: All-or-nothing execution with database transactions
- **Partial Failure Handling**: Continue-on-error mode for resilient operations
- **Progress Tracking**: Detailed results for each operation
- **Validation**: Pre-execution validation to catch errors early
- **Configurable Limits**: Max batch size controls

### API Endpoints

#### Bulk Create Users
```http
POST /api/v1/batch/users/create
Content-Type: application/json

{
  "users": [
    {
      "email": "user1@example.com",
      "name": "John Doe",
      "password": "SecureP@ss123"
    },
    {
      "email": "user2@example.com",
      "name": "Jane Smith",
      "password": "SecureP@ss456"
    }
  ]
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "total": 2,
    "successful": 2,
    "failed": 0,
    "results": [
      {
        "id": "user-0",
        "success": true,
        "data": { "id": "uuid", "email": "user1@example.com" }
      },
      {
        "id": "user-1",
        "success": true,
        "data": { "id": "uuid", "email": "user2@example.com" }
      }
    ]
  }
}
```

#### Bulk Update User Status
```http
POST /api/v1/batch/users/update-status
Content-Type: application/json

{
  "updates": [
    { "userId": "uuid-1", "status": "ACTIVE" },
    { "userId": "uuid-2", "status": "SUSPENDED" }
  ]
}
```

#### Bulk Delete Users
```http
POST /api/v1/batch/users/delete
Content-Type: application/json

{
  "userIds": ["uuid-1", "uuid-2", "uuid-3"]
}
```

### Batch Operation Options

Behaviour is fixed per endpoint rather than configured per request: each batch runs item by item, records a result for every item (`continue on error`), and returns `207 Multi-Status` when any item failed. Size limits are validated on the request struct (`#[validate(length(min = 1, max = 500))]`). Per-item failures never abort the rest of the batch.

### Status Codes

- **200 OK**: All operations successful
- **207 Multi-Status**: Partial success (some operations failed)
- **400 Bad Request**: Invalid request format
- **500 Internal Server Error**: Batch operation failed

### Best Practices

1. **Batch Size**: Keep batches under 500 items for optimal performance
2. **Validation**: Enable `validateBeforeExecute` for critical operations
3. **Error Handling**: Use `continueOnError: true` for non-critical operations
4. **Monitoring**: Log all batch operations for audit trails
5. **Rate Limiting**: Implement per-user rate limits for batch endpoints

---

## Reporting System

### Features
- **Multiple Report Types**: User activity, webhook delivery, system metrics, audit logs
- **Export Formats**: CSV, Excel (XLSX), JSON, PDF (planned)
- **Streaming Support**: Memory-efficient processing for large datasets
- **Flexible Filtering**: Date ranges, user IDs, status filters
- **Scheduled Reports**: Background report generation (planned)

### Report Types

#### 1. User Activity Report
Tracks user registrations, updates, and status changes.

```http
GET /api/v1/reports/user-activity?startDate=2025-01-01&endDate=2025-12-31
```

**Query Parameters:**
- `startDate`: ISO 8601 date (default: 30 days ago)
- `endDate`: ISO 8601 date (default: today)
- `userId`: Filter by specific user
- `status`: Filter by user status

**Response:**
```json
{
  "success": true,
  "data": {
    "recordCount": 150,
    "records": [
      {
        "userId": "uuid",
        "email": "user@example.com",
        "name": "John Doe",
        "status": "ACTIVE",
        "createdAt": "2025-01-15T10:30:00Z",
        "lastUpdated": "2025-12-01T14:20:00Z"
      }
    ]
  }
}
```

#### 2. Webhook Delivery Report
Monitors webhook delivery status and performance.

```http
GET /api/v1/reports/webhook-delivery?startDate=2025-12-01&status=PENDING
```

**Query Parameters:**
- `startDate`: ISO 8601 date (default: 7 days ago)
- `endDate`: ISO 8601 date (default: today)
- `status`: Filter by delivery status (PENDING, SUCCESS, FAILED)

**Response:**
```json
{
  "success": true,
  "data": {
    "recordCount": 42,
    "records": [
      {
        "deliveryId": "uuid",
        "subscriptionId": "uuid",
        "eventType": "user.created",
        "status": "SUCCESS",
        "httpStatus": 200,
        "attempts": 1,
        "createdAt": "2025-12-03T08:15:00Z",
        "deliveredAt": "2025-12-03T08:15:02Z"
      }
    ]
  }
}
```

#### 3. System Metrics Report
Provides real-time system statistics.

```http
GET /api/v1/reports/system-metrics
```

**Response:**
```json
{
  "success": true,
  "data": {
    "recordCount": 4,
    "metrics": [
      { "metric": "Total Users", "value": 1250, "timestamp": "2025-12-03T12:00:00Z" },
      { "metric": "Active Users", "value": 890, "timestamp": "2025-12-03T12:00:00Z" },
      { "metric": "Webhook Subscriptions", "value": 45, "timestamp": "2025-12-03T12:00:00Z" },
      { "metric": "Pending Webhook Deliveries", "value": 12, "timestamp": "2025-12-03T12:00:00Z" }
    ]
  }
}
```

### Export Report (File Download)

#### Generate Report
```http
POST /api/v1/reports/generate
Content-Type: application/json

{
  "type": "USER_ACTIVITY",
  "format": "EXCEL",
  "filters": {
    "startDate": "2025-01-01T00:00:00Z",
    "endDate": "2025-12-31T23:59:59Z",
    "status": "ACTIVE"
  },
  "includeHeaders": true
}
```

**Response:**
```json
{
  "success": true,
  "data": {
    "id": "report_1733240000_abc123",
    "type": "USER_ACTIVITY",
    "format": "EXCEL",
    "status": "COMPLETED",
    "recordCount": 150,
    "url": "data:application/vnd.openxmlformats-officedocument.spreadsheetml.sheet;base64,...",
    "generatedAt": "2025-12-03T12:00:00Z"
  }
}
```

#### Stream Report (Large Datasets)
```http
GET /api/v1/reports/stream?type=USER_ACTIVITY&format=csv&startDate=2025-01-01
```

**Response Headers:**
```
Content-Type: text/csv
Content-Disposition: attachment; filename="report-USER_ACTIVITY-1733240000.csv"
```

### Report Formats

| Format | Extension | MIME Type | Best For |
|--------|-----------|-----------|----------|
| CSV | `.csv` | `text/csv` | Simple data, Excel import |
| Excel | `.xlsx` | `application/vnd.openxmlformats-officedocument.spreadsheetml.sheet` | Formatted reports, charts |
| JSON | `.json` | `application/json` | API consumption, data processing |
| PDF | `.pdf` | `application/pdf` | Printable reports (planned) |

### Best Practices

1. **Use Streaming**: For reports > 10,000 records, use `/reports/stream`
2. **Filter Data**: Always specify date ranges to limit dataset size
3. **Cache Reports**: Store generated reports for reuse (24-hour TTL recommended)
4. **Schedule Reports**: Use scheduled reports for regular reporting needs
5. **Rate Limiting**: Implement per-user rate limits (e.g., 10 reports/hour)
6. **Authentication**: All report endpoints require admin authentication
7. **Audit Logging**: Log all report generation requests

---

## Integration Examples

### React Frontend (Report Download)

```typescript
import axios from 'axios';

async function downloadReport(type: string, format: string) {
  const response = await axios.post('/api/v1/reports/generate', {
    type,
    format,
    filters: {
      startDate: new Date(Date.now() - 30 * 24 * 60 * 60 * 1000),
      endDate: new Date(),
    },
    includeHeaders: true,
  });

  const { url } = response.data.data;
  
  // Download the file
  const link = document.createElement('a');
  link.href = url;
  link.download = `report-${type}-${Date.now()}.${format.toLowerCase()}`;
  link.click();
}
```

### Streaming Report to File

```typescript
async function streamReportToFile(type: string, format: string) {
  const response = await fetch(
    `/api/v1/reports/stream?type=${type}&format=${format}`,
    {
      headers: {
        'Authorization': `Bearer ${token}`,
      },
    }
  );

  const blob = await response.blob();
  const url = URL.createObjectURL(blob);
  
  const link = document.createElement('a');
  link.href = url;
  link.download = `report-${type}.${format}`;
  link.click();
  
  URL.revokeObjectURL(url);
}
```

### Batch Operation with Progress Tracking

```typescript
async function bulkUpdateUsers(updates: Array<{userId: string, status: string}>) {
  const response = await axios.post('/api/v1/batch/users/update-status', {
    updates,
  });

  const { total, successful, failed, results } = response.data.data;
  
  console.log(`Completed: ${successful}/${total} successful, ${failed} failed`);
  
  // Handle partial failures
  if (failed > 0) {
    const failedItems = results.filter(r => !r.success);
    console.error('Failed items:', failedItems);
  }
}
```

---

## Performance Considerations

### Batch Operations
- **Small Batches**: < 100 items → ~100-200ms
- **Medium Batches**: 100-500 items → ~500ms-2s
- **Large Batches**: 500-1000 items → ~2-5s
- **Database Transactions**: Add 10-20% overhead

### Report Generation
- **JSON (API)**: Fastest, < 1s for 10k records
- **CSV**: Fast, ~2-3s for 10k records
- **Excel**: Slower, ~5-10s for 10k records (includes styling)
- **Streaming**: Memory-efficient, processes 100 records at a time

### Scaling Recommendations
1. Use background jobs for reports > 50k records
2. Implement Redis caching for frequently accessed reports
3. Use database read replicas for report queries
4. Shard batch operations across multiple workers
5. Monitor database connection pool saturation

---

## Security Considerations

### Authentication & Authorization
- All batch and reporting endpoints require an admin-tier token and the matching permission (`batch:write`, `report:view`, `report:export`)
- Use JWT authentication with short expiry (access tokens last 1 hour; logout invalidates all sessions)
- Implement refresh token rotation
- Log all administrative actions

### Input Validation
- Validate all batch items before processing
- Use bound SQL parameters for every value; exports neutralise spreadsheet formula prefixes
- Enforce maximum batch sizes (hard limit: 500 per bulk request, 100 for custom batches)
- Validate date ranges (max 1 year)

### Rate Limiting
- Batch operations: 10 requests/hour per user
- Report generation: 20 requests/hour per user
- Report streaming: 50 requests/hour per user

### Audit Logging
Log the following for compliance:
- User ID and timestamp
- Operation type (batch/report)
- Data accessed (record counts, filters)
- Success/failure status
- IP address and user agent

---

## Error Handling

### Batch Operation Errors

**400 Bad Request:**
```json
{
  "success": false,
  "error": "No users provided"
}
```

**207 Multi-Status:**
```json
{
  "success": false,
  "data": {
    "total": 10,
    "successful": 8,
    "failed": 2,
    "results": [
      { "id": "user-7", "success": false, "error": "Email already exists" },
      { "id": "user-9", "success": false, "error": "Invalid password format" }
    ]
  }
}
```

### Report Generation Errors

**500 Internal Server Error:**
```json
{
  "success": false,
  "error": "Report generation failed: Database connection timeout"
}
```

**Report Status (FAILED):**
```json
{
  "success": true,
  "data": {
    "id": "report_123",
    "status": "FAILED",
    "error": "Query timeout after 30 seconds"
  }
}
```

---

## Future Enhancements

### Planned Features
1. **Scheduled Reports**: Cron-based report generation with email delivery
2. **PDF Reports**: Formatted PDF generation with charts
3. **Report Templates**: Customizable report layouts
4. **Batch Validation Rules**: Custom validation per operation type
5. **Progress Webhooks**: Real-time progress updates for long-running batches
6. **Report Caching**: Redis-based caching layer
7. **Audit Log Report**: Dedicated audit trail reporting
8. **Custom Report Builder**: UI-based report query builder

### Migration Path
- All existing endpoints remain backward compatible
- New features will use `/api/v2/` endpoints
- Deprecation notices provided 6 months in advance
