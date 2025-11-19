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
    pub purge: Option<u8>,
    pub http_referrer: Option<String>,
}

impl QueryParameters {
    /// Sanitizes the project parameter
    pub fn project(&self) -> Option<String> {
        let project = self.project.as_ref()?;
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_project() {
        let params = QueryParameters {
            project: Some("ProjectName".to_string()),
            ..Default::default()
        };
        assert_eq!(params.project(), Some("projectname".to_string()));
    }

    #[test]
    fn test_project_none() {
        let params = QueryParameters {
            project: None,
            ..Default::default()
        };
        assert_eq!(params.project(), None);
    }

    #[test]
    fn test_project_empty() {
        let params = QueryParameters {
            project: Some("".to_string()),
            ..Default::default()
        };
        assert_eq!(params.project(), None);
    }

    #[test]
    fn test_project_whitespace() {
        let params = QueryParameters {
            project: Some("  ProjectName  ".to_string()),
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
