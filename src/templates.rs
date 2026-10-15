use crate::query_parameters::QueryParameters;
use anyhow::{Result, anyhow};
use moka::future::Cache;
use std::{
    fmt,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::sync::Semaphore;

/// Per the Wikimedia User-Agent policy, which throttles generic agents harder:
/// <https://foundation.wikimedia.org/wiki/Policy:Wikimedia_Foundation_User-Agent_Policy>
const HTTP_USER_AGENT: &str = "GeoHack/2.0 (https://geohack.toolforge.org; magnusmanske@proton.me)";

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

/// How a template request was answered: `fresh` or `stale` from the cache
/// (a stale one is refreshed in the background), or `fetched` on a cold cache.
fn record_lookup(result: &'static str) {
    wikimisc::metrics::counter!("geohack_template_lookups_total", "result" => result).increment(1);
}

/// Outcome of fetching a template from the wikis (cold cache or background
/// refresh): `ok`, `fallback` (to the English template), `missing` (neither
/// wiki has it), or `failed` (a wiki could not be reached or errored).
fn record_fetch(result: &'static str) {
    wikimisc::metrics::counter!("geohack_template_fetches_total", "result" => result).increment(1);
}

/// The requested template exists neither on the requested wiki nor on enwiki,
/// e.g. for an unknown `globe:`. Caused by the request, not by GeoHack.
#[derive(Debug)]
pub struct TemplateMissing {
    pagename: String,
}

impl fmt::Display for TemplateMissing {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} does not exist", self.pagename)
    }
}

impl std::error::Error for TemplateMissing {}

/// A cached template plus the instant at which it should be revalidated.
/// `html` is `None` for a template that is known not to exist, so that
/// requests for it do not hit the wikis every time.
#[derive(Debug)]
struct CachedTemplate {
    html: Option<Arc<str>>,
    stale_at: Instant,
    /// Set while a background refresh for this entry is in flight, so that
    /// concurrent requests trigger at most one upstream fetch.
    refreshing: AtomicBool,
}

impl CachedTemplate {
    fn new(html: Option<Arc<str>>, fresh_for: Duration) -> Arc<Self> {
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

    fn pagename(&self) -> String {
        let mut pagename = "Template:GeoTemplate".to_string();
        if !self.globe.is_empty() && self.globe != "earth" {
            pagename.push('/');
            pagename.push_str(&urlencoding::encode(&self.globe));
        }
        if self.sandbox {
            pagename += "/sandbox";
        }
        pagename
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
        // not cache errors; a missing template is not an error and is cached.
        // Only a cold cache waits for the network.
        let fetched = AtomicBool::new(false);
        let entry = self
            .cache
            .try_get_with(cache_key, async {
                fetched.store(true, Ordering::Relaxed);
                self.fetch_template(&key)
                    .await
                    .map(|html| CachedTemplate::new(html, FRESH_FOR))
            })
            .await
            .map_err(|error| anyhow!("Failed to load GeoTemplate: {error}"))?;
        record_lookup(if fetched.load(Ordering::Relaxed) {
            "fetched"
        } else if entry.is_stale() {
            "stale"
        } else {
            "fresh"
        });

        // Stale-while-revalidate: answer from the cache immediately and pull a
        // fresh copy in the background, so no user request ever pays for the
        // upstream round-trip once the template has been seen.
        if entry.is_stale() && entry.claim_refresh() {
            self.spawn_refresh(key.clone(), Arc::clone(&entry));
        }

        entry.html.clone().ok_or_else(|| {
            TemplateMissing {
                pagename: key.pagename(),
            }
            .into()
        })
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
                            CachedTemplate::new(stale.html.clone(), RETRY_AFTER),
                        )
                        .await;
                    stale.release_refresh();
                }
            }
        });
    }

    /// Fetches the template from the requested wiki, falling back to enwiki.
    /// `Ok(None)` means both wikis answered 404, i.e. the template does not exist.
    async fn fetch_template(&self, key: &TemplateKey) -> Result<Option<Arc<str>>> {
        let TemplateKey {
            language, project, ..
        } = key;
        let pagename = key.pagename();
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

        let missing_on_requested_wiki = match self.fetch_page(&request_url).await {
            Ok(Some(html)) => {
                record_fetch("ok");
                return Ok(Some(html));
            }
            Ok(None) => true,
            Err(error) => {
                tracing::info!(%error, %request_url, "GeoTemplate fetch failed");
                false
            }
        };

        // Fall back to the English template, localized via uselang, when the
        // wiki has no GeoTemplate of its own.
        tracing::info!(language, %request_url, "GeoTemplate unavailable, falling back to en");
        let request_url_fallback = format!(
            "https://en.wikipedia.org/w/index.php?title={pagename}&uselang={language}&useskin=monobook"
        );
        let error = match self.fetch_page(&request_url_fallback).await {
            Ok(Some(html)) => {
                record_fetch("fallback");
                return Ok(Some(html));
            }
            // Only a definite 404 from both wikis is cached as missing; after
            // any other failure the template might well exist.
            Ok(None) if missing_on_requested_wiki => {
                record_fetch("missing");
                return Ok(None);
            }
            Ok(None) => anyhow!("not on en, and {language} could not be checked"),
            Err(error) => error,
        };
        record_fetch("failed");
        Err(error.context(format!(
            "Could not fetch {pagename} (tried {language} and en)"
        )))
    }

    /// Fetches a page; `Ok(None)` if it does not exist (HTTP 404). MediaWiki
    /// answers that with a "page does not exist" body, which must not be used
    /// as a template. Transport errors and other statuses are errors.
    async fn fetch_page(&self, url: &str) -> Result<Option<Arc<str>>> {
        let response = self.client.get(url).send().await?;
        if response.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(None);
        }
        let html = response.error_for_status()?.text().await?;
        Ok(Some(html.into()))
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
                .insert(
                    key.to_string(),
                    CachedTemplate::new(Some(html.into()), FRESH_FOR),
                )
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
        let entry = CachedTemplate::new(Some("x".into()), Duration::ZERO);
        assert!(entry.is_stale());
        assert!(entry.claim_refresh());
        assert!(!entry.claim_refresh());
        entry.release_refresh();
        assert!(entry.claim_refresh());
    }

    #[test]
    fn test_pagename() {
        let key = TemplateKey {
            language: "de".to_string(),
            globe: "earth".to_string(),
            sandbox: false,
            project: None,
        };
        assert_eq!(key.pagename(), "Template:GeoTemplate");
        assert_eq!(
            TemplateKey {
                globe: "mars".to_string(),
                sandbox: true,
                ..key
            }
            .pagename(),
            "Template:GeoTemplate/mars/sandbox"
        );
    }

    #[tokio::test]
    async fn test_cached_missing_template_is_reported_without_fetching() {
        let templates = Templates::default();
        let key = TemplateKey {
            language: "ar".to_string(),
            globe: "export".to_string(),
            sandbox: false,
            project: None,
        };
        templates
            .cache
            .insert(key.cache_key(), CachedTemplate::new(None, FRESH_FOR))
            .await;
        let query = QueryParameters::new_for_test("1_N_2_E_globe:export", None);
        let error = templates
            .load("ar", "export", &query, false)
            .await
            .expect_err("missing template must be an error");
        assert_eq!(
            error
                .downcast_ref::<TemplateMissing>()
                .map(ToString::to_string),
            Some("Template:GeoTemplate/export does not exist".to_string())
        );
    }

    #[test]
    fn test_fresh_entry_is_not_stale() {
        assert!(!CachedTemplate::new(Some("x".into()), FRESH_FOR).is_stale());
    }
}
