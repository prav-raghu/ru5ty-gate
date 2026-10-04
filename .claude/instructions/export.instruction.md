# Export Functionality Documentation

## Overview

The export system provides CSV and Excel export capabilities with streaming support for handling large datasets efficiently across all backend services.

## Crate: ru5ty-gate-export

Located in `common/export`, this crate provides:

- **CsvExporter**: RFC 4180 compliant CSV generation (`csv` crate)
- **ExcelExporter**: XLSX generation with styled headers (`rust_xlsxwriter`)
- **ExportRow**: `serde_json::Map<String, Value>` — one row, keyed by column name
- **Streaming helpers**: `header_chunk` and `row_chunk` for incremental CSV output

## Features

### CSV Export

- Custom delimiter
- UTF-8 BOM support (opens correctly in Excel)
- Custom or inferred headers
- Chunked output for large datasets
- Spreadsheet formula neutralisation: text cells starting with `=`, `+`, `-`, `@`, tab or carriage return are prefixed with `'` so a malicious value cannot execute as a formula when the file is opened. Numbers are untouched.

### Excel Export

- XLSX format (Excel 2007+)
- Styled headers (bold, colored background)
- Frozen header row and auto-filter
- Custom sheet name and column widths

## Usage Examples

### Basic Buffer Export (Small Datasets)

```rust
let exporter = CsvExporter::new()
    .with_headers(vec!["id".to_owned(), "email".to_owned()])
    .with_bom(true);
let bytes = exporter.export_to_bytes(&rows)?;

let workbook = ExcelExporter::new()
    .with_sheet_name("Users")
    .with_headers(vec!["id".to_owned(), "email".to_owned()])
    .export_to_bytes(&rows)?;
```

### Streaming Export (Large Datasets)

```rust
let exporter = CsvExporter::new().with_bom(true);
let first = exporter.header_chunk(&headers)?;
let next = exporter.row_chunk(&headers, &row)?;
```

`customer-api`'s `ExportController::export_users_stream` builds a `futures_util::stream::unfold` over a pager state (`offset`, `header_sent`, `done`), emits the header chunk first, then one chunk per database page of 100 rows, and hands the stream to `Body::from_stream`. Memory stays flat regardless of table size. Excel cannot be streamed, so that path pages the data into memory up to a hard cap (`SPREADSHEET_LIMIT`, 100,000 rows).

### Controller Implementation

```rust
pub async fn export_users_buffer(
    State(state): State<AppState>,
    ApiQuery(query): ApiQuery<ExportQuery>,
) -> Result<Response, AppError> {
    let users = state.services.user.export_users(0, BUFFER_LIMIT).await?;
    let rows: Vec<ExportRow> = users.iter().map(to_row).collect();
    let bytes = encode(query.format, &rows)?;
    Ok(attachment(query.format, "users", Body::from(bytes)))
}
```

`ExportQuery.format` is an enum (`csv` default, `excel`) so unknown formats fail validation with a 400. `ExportFormat::content_type()` and `extension()` supply the headers for `Content-Type` and `Content-Disposition`.

## API Routes

### Example Routes (customer-api, authenticated)

```
GET /api/v1/users/export/stream?format=csv      streamed, recommended for large datasets
GET /api/v1/users/export/stream?format=excel    paged into memory, capped
GET /api/v1/users/export?format=csv             buffered, first 1,000 rows
GET /api/v1/users/export?format=excel           buffered, first 1,000 rows
```

## Configuration Options

### CSV Options

```rust
CsvExporter::new()
    .with_headers(headers)
    .with_delimiter(b';')
    .with_bom(true)
```

### Excel Options

```rust
ExcelExporter::new()
    .with_sheet_name("Users")
    .with_headers(headers)
    .with_column_widths(vec![36.0, 30.0, 20.0, 24.0])
```

## Performance Considerations

### When to Use Buffer Export

- Small datasets (< 10,000 records)
- Need to return a `Content-Length` header
- Simple data that fits in memory

### When to Use Streaming Export

- Large datasets (> 10,000 records)
- Real-time data generation
- Memory constraints
- Background jobs

## Integration with Services

### Customer API

Export user data, orders, customer reports

### Admin API

Export system logs, analytics, audit trails (reporting endpoints use the same exporters)

### Schedule API

Export appointment history, job results

## Example: Adding Export to Any Service

1. Add `ru5ty-gate-export` and `futures-util` to the service's `Cargo.toml`
2. Add `ExportFormat` and `ExportQuery` in `schemas/export_schema.rs` (copy from `customer-api`)
3. Add a `{entity}_export_controller.rs` with a `to_row` function mapping the entity DTO to an `ExportRow` and the buffer and stream handlers above
4. Add an `export_{entity}(offset, limit)` method to the entity's service that pages with `ORDER BY created_at, id LIMIT $1 OFFSET $2`
5. Register the routes in a `{entity}_export_route.rs` `protected()` router with a permission layer
6. Add tests: headers and BOM, a streamed response with more than one page, and a 401 without a token

## Best Practices

1. **Always use streaming for large datasets** (> 10,000 records)
2. **Set appropriate headers**:
   - Content-Type
   - Content-Disposition (with filename)
3. **Return `AppError` for failures** — never panic inside a stream; on a mid-stream error end the stream with an `io::Error`
4. **Use pagination in data sources** for streaming
5. **Test with production-sized datasets**
6. **Rate limit export endpoints** with the `sensitive_endpoints` policy where data is large or sensitive
7. **Add authentication and a permission layer** to protect sensitive data
8. **Log export operations** for audit trails

## Dependencies

- **csv**: CSV generation (RFC 4180 compliant)
- **rust_xlsxwriter**: Excel file generation
- **futures-util**: `stream::unfold` for streaming bodies in the services

## Testing

```rust
let bytes = CsvExporter::new().export_to_bytes(&rows).unwrap();
let text = String::from_utf8(bytes).unwrap();
assert_eq!(text, "active,id,name\ntrue,1,\"Pat, Jr.\"\nfalse,2,\n");
```

See `common/export/tests/export.rs` for the full set.
