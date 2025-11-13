use serde::Deserialize;

/// The (potential) URL parameters for the geohack.php endpoint.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct QueryParameters {
    pub language: Option<String>,
    pub pagename: Option<String>,
    pub params: String,
    pub default: Option<String>,
    pub dim: Option<String>,
    pub globe: Option<String>,
    pub region: Option<String>,
    pub scale: Option<String>,
    #[serde(rename = "type")]
    pub typename: Option<String>,
    pub zoom: Option<String>,
    pub project: Option<String>,
    pub title: Option<String>,
    pub sandbox: Option<u8>,
    pub http_referrer: Option<String>,
}

impl QueryParameters {
    /// Sanitizes the project parameter
    pub fn project(&self) -> Option<String> {
        match &self.project {
            Some(project) => {
                if project.trim().is_empty() {
                    None
                } else {
                    Some(project.trim().to_ascii_lowercase())
                }
            }
            None => None,
        }
    }

    pub fn sandbox(&self) -> bool {
        self.sandbox == Some(1)
    }
}
