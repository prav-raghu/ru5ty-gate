#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub service: &'static str,
    pub version: &'static str,
    pub port: u16,
    pub production: bool,
    pub docs_path: Option<&'static str>,
}

impl ServerInfo {
    pub fn new(service: &'static str, version: &'static str, port: u16, production: bool) -> Self {
        Self {
            service,
            version,
            port,
            production,
            docs_path: None,
        }
    }

    #[must_use]
    pub fn with_docs(mut self, docs_path: &'static str) -> Self {
        self.docs_path = Some(docs_path);
        self
    }

    pub fn environment(&self) -> &'static str {
        if self.production {
            "production"
        } else {
            "development"
        }
    }

    pub fn local_url(&self) -> String {
        format!("http://localhost:{}", self.port)
    }

    pub fn docs_url(&self) -> Option<String> {
        if self.production {
            return None;
        }
        self.docs_path
            .map(|path| format!("{}{path}", self.local_url()))
    }
}
