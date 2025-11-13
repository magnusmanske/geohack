use crate::GehohackParameters;
use anyhow::Result;
use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::RwLock;

const HTTP_USER_AGENT: &str = "GeoHack/2.0";
const CACHE_DURATION: u64 = 60 * 60;

#[derive(Debug, Clone, Default)]
pub struct Template {
    html: String,
    expires: Option<Instant>,
}

#[derive(Debug, Clone, Default)]
pub struct Templates {
    templates: Arc<RwLock<HashMap<String, Template>>>,
}

impl Templates {
    pub async fn load(
        &self,
        language: &str,
        globe: &str,
        query: &GehohackParameters,
    ) -> Result<String> {
        // TODO proper caching
        let use_sandbox = query.sandbox();
        let use_project = query.project();

        // Try cache
        let caching_key = format!("{language}-{globe}-{use_sandbox}-{use_project:?}");
        if let Some(template) = self.templates.read().await.get(&caching_key)
            && let Some(expires) = &template.expires
            && expires > &Instant::now()
        {
            return Ok(template.html.clone());
        }

        let client = Self::get_reqwest_client()?;

        let mut pagename = "Template:GeoTemplate".to_string();
        if !globe.is_empty() && globe != "earth" {
            pagename.push('/');
            pagename.push_str(&globe.replace("&", "%26"));
        }
        if use_sandbox {
            pagename += "/sandbox";
        }
        let request_url = if let Some(project) = use_project {
            format!(
                "http://meta.wikimedia.org/w/index.php?title={pagename}/{project}&useskin=monobook"
            )
        } else {
            format!("http://{language}.wikipedia.org/w/index.php?title={pagename}&useskin=monobook")
        };

        if let Ok(response) = client.get(&request_url).send().await
            && let Ok(html) = response.text().await
        {
            self.set_template(&caching_key, &html).await?;
            return Ok(html);
        }

        // Fallback
        let request_url = format!(
            "http://en.wikipedia.org/w/index.php?title={pagename}&uselang={language}&useskin=monobook"
        );
        let response = client.get(&request_url).send().await?;
        let html = response.text().await?;
        self.set_template(&caching_key, &html).await?;
        Ok(html)
    }

    async fn set_template(&self, caching_key: &str, html: &str) -> Result<()> {
        self.templates.write().await.insert(
            caching_key.to_string(),
            Template {
                html: html.to_string(),
                expires: Some(Instant::now() + Duration::from_secs(CACHE_DURATION)),
            },
        );
        Ok(())
    }

    fn get_reqwest_client() -> Result<reqwest::Client> {
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent(HTTP_USER_AGENT)
            .build()?;
        Ok(client)
    }
}
