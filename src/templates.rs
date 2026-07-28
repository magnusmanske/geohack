use crate::query_parameters::QueryParameters;
use anyhow::{Result, anyhow};
use moka::future::Cache;
use std::{sync::Arc, time::Duration};

const HTTP_USER_AGENT: &str = "GeoHack/2.0";
const CACHE_DURATION_SEC: u64 = 60 * 60; // 1h
const CACHE_MAX_ENTRIES: u64 = 100;

#[derive(Debug, Clone)]
pub struct Templates {
    // Bounded + TTL'd cache. The key contains user-supplied input (project),
    // so it must not be allowed to grow without limit.
    cache: Cache<String, Arc<str>>,
    client: reqwest::Client,
}

impl Default for Templates {
    fn default() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(CACHE_MAX_ENTRIES)
                .time_to_live(Duration::from_secs(CACHE_DURATION_SEC))
                .build(),
            client: Self::build_reqwest_client().expect("Failed to build reqwest client"),
        }
    }
}

impl Templates {
    pub async fn load(
        &self,
        language: &str,
        globe: &str,
        query: &QueryParameters,
        purge_cache: bool,
    ) -> Result<Arc<str>> {
        let use_sandbox = query.sandbox();
        let use_project = query.project();
        let caching_key = format!("{language}-{globe}-{use_sandbox}-{use_project:?}");

        if purge_cache {
            self.cache.invalidate(&caching_key).await;
        }

        // try_get_with coalesces concurrent fetches for the same key and does
        // not cache errors
        self.cache
            .try_get_with(
                caching_key,
                self.fetch_template(language, globe, use_sandbox, use_project.as_deref()),
            )
            .await
            .map_err(|error| anyhow!("Failed to load GeoTemplate: {error}"))
    }

    async fn fetch_template(
        &self,
        language: &str,
        globe: &str,
        use_sandbox: bool,
        project: Option<&str>,
    ) -> Result<Arc<str>> {
        let mut pagename = "Template:GeoTemplate".to_string();
        if !globe.is_empty() && globe != "earth" {
            pagename.push('/');
            pagename.push_str(&urlencoding::encode(globe));
        }
        if use_sandbox {
            pagename += "/sandbox";
        }
        let request_url = if let Some(project) = project {
            let project = urlencoding::encode(project);
            format!(
                "https://meta.wikimedia.org/w/index.php?title={pagename}/{project}&useskin=monobook"
            )
        } else {
            format!(
                "https://{language}.wikipedia.org/w/index.php?title={pagename}&useskin=monobook"
            )
        };

        if let Ok(response) = self.client.get(&request_url).send().await
            && let Ok(html) = response.text().await
        {
            return Ok(html.into());
        }

        // Fallback
        let request_url_fallback = format!(
            "https://en.wikipedia.org/w/index.php?title={pagename}&uselang={language}&useskin=monobook"
        );
        let response = self.client.get(&request_url_fallback).send().await?;
        let html = response.text().await?;
        Ok(html.into())
    }

    fn build_reqwest_client() -> Result<reqwest::Client> {
        let client = reqwest::ClientBuilder::new()
            .timeout(Duration::from_secs(60))
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent(HTTP_USER_AGENT)
            .build()?;
        Ok(client)
    }

    /// ONLY TO BE USED FOR INTERNAL TESTING PURPOSES
    pub async fn seed_test_cases(&self) -> Result<()> {
        let test_cases = [
            (
                "en--false-None",
                include_str!("../test_data/en--false-None.html"),
            ),
            (
                "en-ganymede-false-None",
                include_str!("../test_data/en-ganymede-false-None.html"),
            ),
            (
                "en-mars-false-None",
                include_str!("../test_data/en-mars-false-None.html"),
            ),
            (
                "en-moon-false-None",
                include_str!("../test_data/en-moon-false-None.html"),
            ),
            (
                "en-venus-false-None",
                include_str!("../test_data/en-venus-false-None.html"),
            ),
        ];
        for (key, html) in test_cases {
            self.cache.insert(key.to_string(), html.into()).await;
        }
        Ok(())
    }
}
