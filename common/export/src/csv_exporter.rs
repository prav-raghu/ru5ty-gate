use crate::csv_cell::csv_cell_text;
use crate::export_error::ExportError;
use crate::export_row::{ExportRow, resolve_headers};

const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

#[derive(Debug, Clone)]
pub struct CsvExporter {
    headers: Option<Vec<String>>,
    delimiter: u8,
    bom: bool,
}

impl Default for CsvExporter {
    fn default() -> Self {
        Self {
            headers: None,
            delimiter: b',',
            bom: false,
        }
    }
}

impl CsvExporter {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_headers(mut self, headers: Vec<String>) -> Self {
        self.headers = Some(headers);
        self
    }

    #[must_use]
    pub fn with_delimiter(mut self, delimiter: u8) -> Self {
        self.delimiter = delimiter;
        self
    }

    #[must_use]
    pub fn with_bom(mut self, bom: bool) -> Self {
        self.bom = bom;
        self
    }

    fn writer(&self) -> csv::Writer<Vec<u8>> {
        csv::WriterBuilder::new()
            .delimiter(self.delimiter)
            .from_writer(Vec::new())
    }

    fn finish(writer: csv::Writer<Vec<u8>>) -> Result<Vec<u8>, ExportError> {
        writer
            .into_inner()
            .map_err(|error| ExportError::Flush(error.to_string()))
    }

    pub fn export_to_bytes(&self, rows: &[ExportRow]) -> Result<Vec<u8>, ExportError> {
        let headers = resolve_headers(self.headers.as_deref(), rows);
        let mut writer = self.writer();
        if !headers.is_empty() {
            writer.write_record(&headers)?;
        }
        for row in rows {
            writer.write_record(
                headers
                    .iter()
                    .map(|key| row.get(key).map_or_else(String::new, csv_cell_text)),
            )?;
        }
        let mut output = Vec::new();
        if self.bom {
            output.extend_from_slice(&BOM);
        }
        output.extend(Self::finish(writer)?);
        Ok(output)
    }

    pub fn header_chunk(&self, headers: &[String]) -> Result<Vec<u8>, ExportError> {
        let mut writer = self.writer();
        writer.write_record(headers)?;
        let mut output = Vec::new();
        if self.bom {
            output.extend_from_slice(&BOM);
        }
        output.extend(Self::finish(writer)?);
        Ok(output)
    }

    pub fn row_chunk(&self, headers: &[String], row: &ExportRow) -> Result<Vec<u8>, ExportError> {
        let mut writer = self.writer();
        writer.write_record(
            headers
                .iter()
                .map(|key| row.get(key).map_or_else(String::new, csv_cell_text)),
        )?;
        Self::finish(writer)
    }
}
