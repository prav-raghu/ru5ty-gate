use metrics::{counter, histogram};

pub fn record_database_query(prefix: &str, operation: &str, seconds: f64) {
    counter!(
        format!("{prefix}database_queries_total"),
        "operation" => operation.to_owned()
    )
    .increment(1);
    histogram!(
        format!("{prefix}database_query_duration_seconds"),
        "operation" => operation.to_owned()
    )
    .record(seconds);
}

pub fn record_cache_operation(prefix: &str, operation: &str, hit: bool, seconds: f64) {
    let outcome = if hit { "hit" } else { "miss" };
    counter!(
        format!("{prefix}cache_operations_total"),
        "operation" => operation.to_owned(),
        "outcome" => outcome
    )
    .increment(1);
    histogram!(
        format!("{prefix}cache_operation_duration_seconds"),
        "operation" => operation.to_owned()
    )
    .record(seconds);
}
