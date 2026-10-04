#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use ru5ty_gate_export::{CsvExporter, ExcelExporter, ExportRow};
use serde_json::json;

fn rows() -> Vec<ExportRow> {
    vec![
        json!({"id": 1, "name": "Pat, Jr.", "active": true})
            .as_object()
            .cloned()
            .unwrap(),
        json!({"id": 2, "name": null, "active": false})
            .as_object()
            .cloned()
            .unwrap(),
    ]
}

#[test]
fn csv_quotes_values_and_writes_headers() {
    let bytes = CsvExporter::new().export_to_bytes(&rows()).unwrap();
    let text = String::from_utf8(bytes).unwrap();

    assert_eq!(text, "active,id,name\ntrue,1,\"Pat, Jr.\"\nfalse,2,\n");
}

#[test]
fn csv_honours_explicit_headers_and_bom() {
    let exporter = CsvExporter::new()
        .with_headers(vec!["name".to_owned(), "id".to_owned()])
        .with_bom(true);

    let bytes = exporter.export_to_bytes(&rows()).unwrap();

    assert_eq!(&bytes[..3], &[0xEF, 0xBB, 0xBF]);
    assert!(
        String::from_utf8(bytes[3..].to_vec())
            .unwrap()
            .starts_with("name,id\n")
    );
}

#[test]
fn csv_streams_header_then_rows() {
    let exporter = CsvExporter::new();
    let headers = vec!["id".to_owned(), "name".to_owned()];
    let all = rows();

    let mut output = exporter.header_chunk(&headers).unwrap();
    for row in &all {
        output.extend(exporter.row_chunk(&headers, row).unwrap());
    }

    assert_eq!(
        String::from_utf8(output).unwrap(),
        "id,name\n1,\"Pat, Jr.\"\n2,\n"
    );
}

#[test]
fn excel_produces_a_zip_container() {
    let bytes = ExcelExporter::new().export_to_bytes(&rows()).unwrap();

    assert_eq!(&bytes[..2], b"PK");
}

#[test]
fn excel_handles_empty_input() {
    let bytes = ExcelExporter::new().export_to_bytes(&[]).unwrap();

    assert_eq!(&bytes[..2], b"PK");
}

#[test]
fn csv_neutralises_spreadsheet_formula_prefixes_in_text_but_not_numbers() {
    let rows = vec![
        json!({"cell": "=SUM(A1:A9)", "amount": -5, "plain": "safe"})
            .as_object()
            .cloned()
            .unwrap(),
        json!({"cell": "@cmd", "amount": 3, "plain": "+1 555"})
            .as_object()
            .cloned()
            .unwrap(),
    ];

    let text = String::from_utf8(CsvExporter::new().export_to_bytes(&rows).unwrap()).unwrap();

    assert_eq!(
        text,
        "amount,cell,plain\n-5,'=SUM(A1:A9),safe\n3,'@cmd,'+1 555\n"
    );
}
