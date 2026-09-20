use crate::query_parameters::QueryParameters;
use anyhow::{Result, anyhow};
use moka::future::Cache;
use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;

const HTTP_USER_AGENT: &str = "GeoHack/2.0";

/// How long a freshly fetched template is served without any revalidation.
const FRESH_FOR: Duration = Duration::from_secs(60 * 60); // 1h
/// How long a template may still be served *while* a refresh runs in the
/// background. Only reached when the upstream wiki keeps failing.
const SERVE_STALE_FOR: Duration = Duration::from_secs(24 * 60 * 60); // 24h
/// Backoff before retrying a background refresh that failed, so a broken
/// upstream is not hammered once per request.
const RETRY_AFTER: Duration = Duration::from_secs(5 * 60); // 5min

const CACHE_MAX_ENTRIES: u64 = 100;
/// Upper bound on concurrent *background* refreshes. Entries populated by the
/// same traffic burst also go stale together, so without this the whole cache
/// would hit the wikis at once. User-facing (cold cache) fetches are not
/// throttled — nobody is waiting on a background refresh.
const MAX_CONCURRENT_REFRESHES: usize = 4;

/// A cached template plus the instant at which it should be revalidated.
#[derive(Debug)]
struct CachedTemplate {
    html: Arc<str>,
    stale_at: Instant,
    /// Set while a background refresh for this entry is in flight, so that
    /// concurrent requests trigger at most one upstream fetch.
    refreshing: AtomicBool,
}

impl CachedTemplate {
    fn new(html: Arc<str>, fresh_for: Duration) -> Arc<Self> {
        Arc::new(Self {
            html,
            stale_at: Instant::now() + fresh_for,
            refreshing: AtomicBool::new(false),
        })
    }

    fn is_stale(&self) -> bool {
        Instant::now() >= self.stale_at
    }

    /// Returns `true` for exactly one caller, which then owns the refresh.
    fn claim_refresh(&self) -> bool {
        self.refreshing
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn release_refresh(&self) {
        self.refreshing.store(false, Ordering::Release);
    }
}

/// Identifies one cacheable template. Owned (not borrowed) so it can be moved
/// into a background refresh task.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct TemplateKey {
    language: String,
    globe: String,
    sandbox: bool,
    project: Option<String>,
}

impl TemplateKey {
    fn cache_key(&self) -> String {
        let Self {
            language,
            globe,
            sandbox,
            project,
        } = self;
        format!("{language}-{globe}-{sandbox}-{project:?}")
    }
}

#[derive(Debug, Clone)]
pub struct Templates {
    // Bounded + TTL'd cache. The key contains user-supplied input (project),
    // so it must not be allowed to grow without limit.
    cache: Cache<String, Arc<CachedTemplate>>,
    client: reqwest::Client,
    refresh_slots: Arc<Semaphore>,
}

impl Default for Templates {
    fn default() -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(CACHE_MAX_ENTRIES)
                .time_to_live(SERVE_STALE_FOR)
                .build(),
            client: Self::build_reqwest_client().expect("Failed to build reqwest client"),
            refresh_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_REFRESHES)),
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
        let key = TemplateKey {
            language: language.to_string(),
            globe: globe.to_string(),
            sandbox: query.sandbox(),
            project: query.project(),
        };
        let cache_key = key.cache_key();

        if purge_cache {
            self.cache.invalidate(&cache_key).await;
        }

        // try_get_with coalesces concurrent fetches for the same key and does
        // not cache errors. Only a cold cache waits for the network.
        let entry = self
            .cache
            .try_get_with(cache_key, async {
                self.fetch_template(&key)
                    .await
                    .map(|html| CachedTemplate::new(html, FRESH_FOR))
            })
            .await
            .map_err(|error| anyhow!("Failed to load GeoTemplate: {error}"))?;

        // Stale-while-revalidate: answer from the cache immediately and pull a
        // fresh copy in the background, so no user request ever pays for the
        // upstream round-trip once the template has been seen.
        if entry.is_stale() && entry.claim_refresh() {
            self.spawn_refresh(key, Arc::clone(&entry));
        }

        Ok(Arc::clone(&entry.html))
    }

    fn spawn_refresh(&self, key: TemplateKey, stale: Arc<CachedTemplate>) {
        let this = self.clone();
        tokio::spawn(async move {
            let cache_key = key.cache_key();
            // Held for the whole fetch; the `refreshing` flag keeps further
            // requests for this key from queueing up behind it.
            let Ok(_permit) = this.refresh_slots.acquire().await else {
                return;
            };
            match this.fetch_template(&key).await {
                Ok(html) => {
                    this.cache
                        .insert(cache_key, CachedTemplate::new(html, FRESH_FOR))
                        .await;
                }
                Err(error) => {
                    tracing::warn!(%error, cache_key, "Background GeoTemplate refresh failed");
                    // Keep serving the stale copy, but back off before retrying.
                    this.cache
                        .insert(
                            cache_key,
                            CachedTemplate::new(Arc::clone(&stale.html), RETRY_AFTER),
                        )
                        .await;
                    stale.release_refresh();
                }
            }
        });
    }

    async fn fetch_template(&self, key: &TemplateKey) -> Result<Arc<str>> {
        let TemplateKey {
            language,
            globe,
            sandbox,
            project,
        } = key;

        let mut pagename = "Template:GeoTemplate".to_string();
        if !globe.is_empty() && globe != "earth" {
            pagename.push('/');
            pagename.push_str(&urlencoding::encode(globe));
        }
        if *sandbox {
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

        if let Some(html) = self.fetch_page(&request_url).await {
            return Ok(html);
        }

        // Fall back to the English template, localized via uselang, when the
        // wiki has no GeoTemplate of its own. MediaWiki answers 404 with a
        // "page does not exist" body, which must not be used as a template.
        tracing::info!(language, %request_url, "GeoTemplate unavailable, falling back to en");
        let request_url_fallback = format!(
            "https://en.wikipedia.org/w/index.php?title={pagename}&uselang={language}&useskin=monobook"
        );
        self.fetch_page(&request_url_fallback)
            .await
            .ok_or_else(|| anyhow!("Could not fetch {pagename} (tried {language} and en)"))
    }

    /// Fetch a page, returning `None` for transport errors and for any
    /// non-success HTTP status (notably 404 for a missing template)
    async fn fetch_page(&self, url: &str) -> Option<Arc<str>> {
        let response = self.client.get(url).send().await.ok()?;
        if !response.status().is_success() {
            return None;
        }
        Some(response.text().await.ok()?.into())
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
            self.cache
                .insert(key.to_string(), CachedTemplate::new(html.into(), FRESH_FOR))
                .await;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_key_is_unchanged() {
        let key = TemplateKey {
            language: "en".to_string(),
            globe: String::new(),
            sandbox: false,
            project: None,
        };
        assert_eq!(key.cache_key(), "en--false-None");
        assert_eq!(
            TemplateKey {
                globe: "mars".to_string(),
                sandbox: true,
                project: Some("wikivoyage".to_string()),
                ..key
            }
            .cache_key(),
            r#"en-mars-true-Some("wikivoyage")"#
        );
    }

    #[test]
    fn test_only_one_caller_claims_a_refresh() {
        let entry = CachedTemplate::new("x".into(), Duration::ZERO);
        assert!(entry.is_stale());
        assert!(entry.claim_refresh());
        assert!(!entry.claim_refresh());
        entry.release_refresh();
        assert!(entry.claim_refresh());
    }

    #[test]
    fn test_fresh_entry_is_not_stale() {
        assert!(!CachedTemplate::new("x".into(), FRESH_FOR).is_stale());
    }
}
