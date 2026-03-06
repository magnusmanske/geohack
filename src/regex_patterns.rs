use regex::Regex;
use std::sync::LazyLock;

/// Regex for fixing language codes - extracts the language prefix
pub static RE_FIX_LANGUAGE_CODE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^([\-a-z]+)").expect("Invalid regex pattern"));

/// Regex for sanitizing HTML - removes script tags
pub static RE_SANITIZE_HTML: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<script.+</script>").expect("Invalid regex pattern"));

/// Regex for making links - checks for special characters
pub static RE_MAKE_LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^0-9A-Za-z_.:;@$!*(),/\\-]").expect("Invalid regex pattern"));

/// Regex for extracting pagename from referrer URL
pub static RE_INIT_FROM_QUERY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"https?://[^/]+/?(?:wiki/|w/index.php\?.*?title=)([^&?#{}\[\]]+)")
        .expect("Invalid regex pattern")
});

/// Regex for replacing Wikipedia language links in HTML
pub static RE_WIKIPEDIA_LANG_LINK: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#" href="(https?:)//([a-z\-]+)?\.wikipedia\.org/wiki/[^"]*"#)
        .expect("Invalid regex pattern")
});
