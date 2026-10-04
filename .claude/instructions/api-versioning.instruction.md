# API Versioning Strategy

## Overview

This monorepo implements an API versioning strategy across all backend services (customer-api, admin-api, schedule-api) to ensure backward compatibility and smooth migration paths. The behaviour lives in one shared middleware, `api_version`, in `common/http/src/middleware/api_version.rs`, applied by every service.

## Version Detection Methods

The API version can be specified in three ways (in order of precedence):

### 1. Custom Header (Recommended)
```bash
curl -H "API-Version: v2" https://api.example.com/users
```

### 2. Accept Header
```bash
curl -H "Accept: application/vnd.api.v2+json" https://api.example.com/users
```

### 3. URL Path (Most Common)
```bash
curl https://api.example.com/api/v2/users
```

When none is present the middleware falls back to `v1`.

## Response Headers

All API responses include version information:

```
X-API-Version: v2
```

For deprecated versions:
```
Deprecation: true
Sunset: 2026-12-31
X-API-Deprecation-Info: v1 will be deprecated on 2026-12-31. Please migrate to v2.
```

## Current Version Status

- **v1**: Currently supported, **deprecated** (sunset: 2026-12-31)
- **v2**: Current stable version

## Usage Examples

### Frontend Integration

#### Axios Configuration
```typescript
import axios from 'axios';

const apiClient = axios.create({
  baseURL: '/api',
  headers: {
    'API-Version': 'v2',
  },
});

apiClient.interceptors.response.use(
  (response) => {
    const version = response.headers['x-api-version'];
    const isDeprecated = response.headers['deprecation'] === 'true';

    if (isDeprecated) {
      console.warn(
        `API version ${version} is deprecated. ` +
        `Sunset date: ${response.headers['sunset']}. ` +
        `Info: ${response.headers['x-api-deprecation-info']}`
      );
    }

    return response;
  },
  (error) => Promise.reject(error)
);
```

#### URL-Based Versioning
```typescript
const response = await fetch('/api/v2/users');
```

### Backend Service URLs

- **Customer API v1**: `http://localhost:4002/api/v1/`
- **Customer API v2**: `http://localhost:4002/api/v2/`
- **Admin API v1**: `http://localhost:4001/api/v1/`
- **Admin API v2**: `http://localhost:4001/api/v2/`
- **Schedule API v1**: `http://localhost:4003/api/v1/`
- **Schedule API v2**: `http://localhost:4003/api/v2/`

## Version Management

### ApiVersionManager Utility

Located in `common/utilities/src/api_version.rs`:

```rust
use ru5ty_gate_utilities::ApiVersionManager;

let versions = ApiVersionManager::default();
versions.is_version_supported("v2");
versions.current_version();
versions.supported_versions();
versions.deprecated_versions();
```

### Adding a New Version

1. **Update the shared middleware** constants in `common/http/src/middleware/api_version.rs` (`SUPPORTED`, and the deprecated version, sunset date and message when an older version is being retired):

```rust
const SUPPORTED: [&str; 3] = ["v1", "v2", "v3"];
const DEPRECATED_VERSION: &str = "v2";
const SUNSET: &str = "2027-06-30";
```

2. **Update `ApiVersionManager::default`** in `common/utilities/src/api_version.rs` so the version list, deprecation flags and current version agree with the middleware.

3. **Create the v3 routes module** in each service:
```
apps/backend/customer-api/src/routes/v3/
├── mod.rs
└── v3_route.rs
```

4. **Nest the routes** in `application.rs`:
```rust
Router::new()
    .nest("/api/v1", V1Routes::register(&self.state))
    .nest("/api/v2", V2Routes::register())
    .nest("/api/v3", V3Routes::register())
```

5. **Add a test** to `common/http/tests` for the new header values and to the service's integration tests for the new prefix.

## Error Handling

### Unsupported Version
```json
{
  "success": false,
  "error": "Unsupported API version",
  "supportedVersions": ["v1", "v2"]
}
```
Status: `400 Bad Request`. This body is the version middleware's contract and deliberately differs from the standard envelope.

## Best Practices

1. **Never break v1 routes** - Keep backward compatibility
2. **Gradual migration** - Give clients 6-12 months notice before deprecation
3. **Monitor usage** - Track which versions are being used
4. **Document changes** - Maintain changelog for each version
5. **Use semantic versioning** - Major version for breaking changes

## Migration Guide

### From v1 to v2

#### Breaking Changes
- TBD based on actual implementation

#### Deprecated Endpoints
- None yet

#### New Features in v2
- Enhanced error handling
- Improved response formats
- Additional validation

### Migration Checklist
- [ ] Review API changelog
- [ ] Update client code to use v2 endpoints
- [ ] Update API-Version header or URL prefix
- [ ] Test all endpoints
- [ ] Monitor for deprecation warnings
- [ ] Remove v1 references before sunset date

## Testing

### Test Version Detection
```bash
# Test header-based
curl -H "API-Version: v2" http://localhost:4002/api/v1/ping

# Test URL-based
curl http://localhost:4002/api/v2/ping

# Test invalid version
curl -H "API-Version: v99" http://localhost:4002/api/v1/ping
```

### Verify Deprecation Headers
```bash
curl -v http://localhost:4002/api/v1/ping | grep -i deprecation
```

## Configuration

Versioning configuration is centralised, not per service:

```
common/http/src/middleware/api_version.rs     detection, headers, supported set
common/utilities/src/api_version.rs   queryable version metadata
```

## Architecture

```
Request → api_version middleware → Version Detection → Route Resolution
                                        ↓
                            Reject unsupported (400)
                                        ↓
                            Execute Handler
                                        ↓
                      Set X-API-Version / deprecation headers
```

## Monitoring & Analytics

Consider tracking:
- Version usage distribution
- Deprecated version usage
- Version migration progress
- Error rates per version

Log the detected version (it is available to handlers as the `ApiVersion` request extension) and aggregate it in your log pipeline.
