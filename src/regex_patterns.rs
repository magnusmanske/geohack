use once_cell::sync::Lazy;
use regex::Regex;

/// Regex for fixing language codes - extracts the language prefix
pub static RE_FIX_LANGUAGE_CODE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^([\-a-z]+)").expect("Invalid regex pattern"));

/// Regex for sanitizing HTML - removes script tags
pub static RE_SANITIZE_HTML: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"<script.+</script>").expect("Invalid regex pattern"));

/// Regex for making links - checks for special characters
pub static RE_MAKE_LINK: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[^0-9A-Za-z_.:;@$!*(),/\\-]").expect("Invalid regex pattern"));

/// Regex for extracting pagename from referrer URL
pub static RE_INIT_FROM_QUERY: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"https?://[^/]+/?(?:wiki/|w/index.php\?.*?title=)([^&?#{}\[\]]+)")
        .expect("Invalid regex pattern")
});

/// Regex for replacing Wikipedia language links in HTML
pub static RE_WIKIPEDIA_LANG_LINK: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#" href="(https?:)//([a-z\-]+)?\.wikipedia\.org/wiki/[^"]*"#)
        .expect("Invalid regex pattern")
});
