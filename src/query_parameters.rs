use serde::Deserialize;

/// The (potential) URL parameters for the geohack.php endpoint.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct QueryParameters {
    language: Option<String>,
    pagename: Option<String>,
    params: String,
    // default: Option<String>,
    // dim: Option<String>,
    // globe: Option<String>,
    // region: Option<String>,
    // scale: Option<String>,
    // #[serde(rename = "type")]
    // typename: Option<String>,
    // zoom: Option<String>,
    #[serde(rename = "project")]
    project_field: Option<String>,
    title: Option<String>,
    sandbox: Option<u8>,
    purge: Option<u8>,
    #[serde(skip)]
    http_referrer: Option<String>,
}

impl QueryParameters {
    pub fn language(&self) -> Option<&str> {
        self.language.as_deref()
    }

    pub fn pagename(&self) -> Option<&str> {
        self.pagename.as_deref()
    }

    pub fn params(&self) -> &str {
        &self.params
    }

    pub fn title(&self) -> Option<&str> {
        self.title.as_deref()
    }

    pub fn http_referrer(&self) -> Option<&str> {
        self.http_referrer.as_deref()
    }

    pub fn set_http_referrer(&mut self, referrer: Option<String>) {
        self.http_referrer = referrer;
    }

    /// Sanitizes the project parameter
    pub fn project(&self) -> Option<String> {
        let project = self.project_field.as_ref()?;
        if project.trim().is_empty() {
            None
        } else {
            Some(project.trim().to_ascii_lowercase())
        }
    }

    /// Using a sandbox page?
    pub fn sandbox(&self) -> bool {
        self.sandbox == Some(1)
    }

    /// Purge cache if requested by user, or if a sandbox is used
    pub fn purge(&self) -> bool {
        self.purge == Some(1) || self.sandbox()
    }

    /// Create a new QueryParameters for testing with params and optional title
    #[cfg(test)]
    pub fn new_for_test(params: &str, title: Option<&str>) -> Self {
        Self {
            params: params.to_string(),
            title: title.map(|s| s.to_string()),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project() {
        let params = QueryParameters {
            project_field: Some("ProjectName".to_string()),
            ..Default::default()
        };
        assert_eq!(params.project(), Some("projectname".to_string()));
    }

    #[test]
    fn test_project_none() {
        let params = QueryParameters {
            project_field: None,
            ..Default::default()
        };
        assert_eq!(params.project(), None);
    }

    #[test]
    fn test_project_empty() {
        let params = QueryParameters {
            project_field: Some("".to_string()),
            ..Default::default()
        };
        assert_eq!(params.project(), None);
    }

    #[test]
    fn test_project_whitespace() {
        let params = QueryParameters {
            project_field: Some("  ProjectName  ".to_string()),
            ..Default::default()
        };
        assert_eq!(params.project(), Some("projectname".to_string()));
    }

    #[test]
    fn test_purge() {
        let params = QueryParameters {
            purge: Some(1),
            ..Default::default()
        };
        assert!(params.purge());
    }

    #[test]
    fn test_purge_none() {
        let params = QueryParameters {
            purge: None,
            sandbox: None,
            ..Default::default()
        };
        assert!(!params.purge());
    }

    #[test]
    fn test_purge_none_sandbox() {
        let params = QueryParameters {
            purge: None,
            sandbox: Some(1),
            ..Default::default()
        };
        assert!(params.purge());
    }

    #[test]
    fn test_sandbox() {
        let params = QueryParameters {
            sandbox: Some(1),
            ..Default::default()
        };
        assert!(params.sandbox());
    }

    #[test]
    fn test_sandbox_none() {
        let params = QueryParameters {
            sandbox: None,
            ..Default::default()
        };
        assert!(!params.sandbox());
    }

    #[test]
    fn test_sandbox_wrong() {
        let params = QueryParameters {
            sandbox: Some(2),
            ..Default::default()
        };
        assert!(!params.sandbox());
    }
}
