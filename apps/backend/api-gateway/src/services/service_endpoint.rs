#[derive(Debug, Clone)]
pub struct ServiceEndpoint {
    pub name: String,
    pub url: String,
    pub health_path: String,
}
