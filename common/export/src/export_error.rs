use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExportError {
    #[error("csv export failed: {0}")]
    Csv(#[from] csv::Error),
    #[error("csv output was not flushed: {0}")]
    Flush(String),
    #[error("excel export failed: {0}")]
    Excel(#[from] rust_xlsxwriter::XlsxError),
}
