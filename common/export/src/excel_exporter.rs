use rust_xlsxwriter::{Color, Format, FormatAlign, Workbook};
use serde_json::Value;

use crate::export_error::ExportError;
use crate::export_row::{ExportRow, cell_text, resolve_headers};

const DEFAULT_COLUMN_WIDTH: f64 = 15.0;
const HEADER_FILL: u32 = 0x00D3_D3D3;

#[derive(Debug, Clone)]
pub struct ExcelExporter {
    sheet_name: String,
    headers: Option<Vec<String>>,
    column_widths: Vec<f64>,
    freeze_header: bool,
    auto_filter: bool,
    style_header: bool,
}

impl Default for ExcelExporter {
    fn default() -> Self {
        Self {
            sheet_name: "Sheet1".to_owned(),
            headers: None,
            column_widths: Vec::new(),
            freeze_header: true,
            auto_filter: true,
            style_header: true,
        }
    }
}

impl ExcelExporter {
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn with_sheet_name(mut self, name: &str) -> Self {
        name.clone_into(&mut self.sheet_name);
        self
    }

    #[must_use]
    pub fn with_headers(mut self, headers: Vec<String>) -> Self {
        self.headers = Some(headers);
        self
    }

    #[must_use]
    pub fn with_column_widths(mut self, widths: Vec<f64>) -> Self {
        self.column_widths = widths;
        self
    }

    pub fn export_to_bytes(&self, rows: &[ExportRow]) -> Result<Vec<u8>, ExportError> {
        let headers = resolve_headers(self.headers.as_deref(), rows);
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        worksheet.set_name(&self.sheet_name)?;

        let header_format = Format::new()
            .set_bold()
            .set_background_color(Color::RGB(HEADER_FILL))
            .set_align(FormatAlign::Center);

        for (column, header) in headers.iter().enumerate() {
            let column = u16::try_from(column).unwrap_or(u16::MAX);
            if self.style_header {
                worksheet.write_string_with_format(0, column, header, &header_format)?;
            } else {
                worksheet.write_string(0, column, header)?;
            }
            let width = self
                .column_widths
                .get(usize::from(column))
                .copied()
                .unwrap_or(DEFAULT_COLUMN_WIDTH);
            worksheet.set_column_width(column, width)?;
        }

        for (index, row) in rows.iter().enumerate() {
            let row_index = u32::try_from(index + 1).unwrap_or(u32::MAX);
            for (column, header) in headers.iter().enumerate() {
                let column = u16::try_from(column).unwrap_or(u16::MAX);
                match row.get(header) {
                    Some(Value::Number(number)) => {
                        worksheet.write_number(
                            row_index,
                            column,
                            number.as_f64().unwrap_or(0.0),
                        )?;
                    }
                    Some(Value::Bool(flag)) => {
                        worksheet.write_boolean(row_index, column, *flag)?;
                    }
                    Some(value) => {
                        worksheet.write_string(row_index, column, cell_text(value))?;
                    }
                    None => {}
                }
            }
        }

        if self.freeze_header {
            worksheet.set_freeze_panes(1, 0)?;
        }
        if self.auto_filter && !headers.is_empty() {
            let last_row = u32::try_from(rows.len()).unwrap_or(u32::MAX);
            let last_column = u16::try_from(headers.len() - 1).unwrap_or(u16::MAX);
            worksheet.autofilter(0, 0, last_row, last_column)?;
        }
        Ok(workbook.save_to_buffer()?)
    }
}
