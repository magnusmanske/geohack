/**
 * Copyright 2006 by <Magnus Manske> <magnusmanske@googlemail.com>
 * Released under GPL
 * Converted to Rust 2005 by <Magnus Manske> <magnusmanske@googlemail.com>
*/
use crate::map_sources::MapSources;
use crate::{GehohackParameters, geo_param::GeoParam};
use anyhow::{Result, anyhow};
use html_escape;
use lazy_static::lazy_static;
use regex::Regex;
use std::collections::HashMap;
use urlencoding;

/// Main GeoHack application struct
pub struct GeoHack {
    pub lang: String,
    params: String,
    pagename: String,
    title: String,
    map_sources: MapSources,
    region_name: Option<String>,
    pub globe: String,
    nlzoom: String,
    page_content: String,
    logo_urls: HashMap<String, String>,
    actions: String,
    languages: String,
}

impl GeoHack {
    /// Create a new GeoHack instance
    pub fn new() -> Result<Self> {
        Ok(GeoHack {
            lang: "en".to_string(),
            params: String::new(),
            pagename: String::new(),
            title: String::new(),
            map_sources: MapSources::default(),
            region_name: None,
            globe: String::new(),
            nlzoom: String::new(),
            page_content: String::new(),
            logo_urls: Self::init_logo_urls(),
            actions: String::new(),
            languages: String::new(),
        })
    }

    /// Initialize logo URLs for different celestial bodies
    fn init_logo_urls() -> HashMap<String, String> {
        let json_text = include_str!("../data/logos.json");
        serde_json::from_str(json_text).expect("Invalid JSON in logos.json")
    }

    /// Get request parameter with default value
    pub fn get_request(
        &self,
        params: &HashMap<String, String>,
        key: &str,
        default: &str,
    ) -> String {
        if let Some(value) = params.get(key) {
            Self::sanitize_html(value)
        } else {
            default.to_string()
        }
    }

    fn sanitize_html(html: &str) -> String {
        let re = Regex::new(r"<script.+</script>").unwrap();
        re.replace_all(&html.replace("\\'", "'"), "").to_string()
    }

    /// Fix language code
    pub fn fix_language_code(&self, lang: &str, default: &str) -> String {
        let lang = lang.trim().to_lowercase();
        let re = Regex::new(r"^([\-a-z]+)").unwrap();
        if let Some(captures) = re.captures(&lang) {
            captures
                .get(1)
                .map_or(default.to_string(), |m| m.as_str().to_string())
        } else {
            default.to_string()
        }
    }

    /// Get a div section from HTML
    pub fn get_div_section(&self, html: &str, node_id: &str, begin: usize) -> String {
        let search_str = format!("<div id=\"{}\"", node_id);
        if let Some(begin_pos) = html[begin..].find(&search_str) {
            let begin = begin + begin_pos;
            let mut end = begin;
            let mut start = begin;

            loop {
                let next_end = html[end + 6..].find("</div>");
                let next_start = html[start + 4..].find("<div");

                match (next_end, next_start) {
                    (Some(e), Some(s)) => {
                        end = end + 6 + e;
                        start = start + 4 + s;
                        if start >= end {
                            break;
                        }
                    }
                    (Some(e), None) => {
                        end = end + 6 + e;
                        break;
                    }
                    _ => return String::new(),
                }
            }

            html[begin..=end + 5].to_string()
        } else {
            String::new()
        }
    }

    /// Make a link for language switching
    pub fn make_link(&self, lang: &str, params: &str, pagename: &str) -> String {
        let mut query = if !pagename.is_empty() {
            format!("&pagename={}", pagename)
        } else {
            String::new()
        };

        // TODO params match characters: %+
        let re = Regex::new(r"[^0-9A-Za-z_.:;@$!*(),/\\-]").unwrap();
        let path = if !re.is_match(params) {
            // Short url
            format!("/{}/{}", lang, params)
        } else {
            query.push_str(&format!("&language={}&params={}", lang, params));
            "/geohack".to_string()
        };

        if !query.is_empty() {
            format!("{}?{}", path, &query[1..])
        } else {
            path
        }
    }

    pub fn init_from_query(&mut self, query: GehohackParameters) -> Result<()> {
        let lang = Self::sanitize_html(&query.language.unwrap_or("en".to_string()));
        let params = Self::sanitize_html(&query.params);
        self.lang = self.fix_language_code(&lang, "");
        self.params = html_escape::encode_text(&params).to_string();

        if self.params.is_empty() {
            return Err(anyhow!(
                "No parameters given (&params= is empty or missing)"
            ));
        }

        // Using REFERER as a last resort for pagename
        let referer = query.http_referrer.unwrap_or_default().clone();
        let re =
            Regex::new(r"https?://[^/]+/?(?:wiki/|w/index.php\?.*?title=)([^&?#{}\[\]]+)").unwrap();
        let ref_match = re.captures(&referer);
        let default_pagename = if let Some(captures) = ref_match {
            urlencoding::decode(captures.get(1).unwrap().as_str())
                .unwrap_or_default()
                .to_string()
        } else {
            String::new()
        };

        let pagename = Self::sanitize_html(&query.pagename.unwrap_or(default_pagename));
        self.pagename = html_escape::encode_text(&pagename).to_string();

        let title = Self::sanitize_html(&query.title.unwrap_or(self.pagename.replace('_', " ")));
        self.title = html_escape::encode_text(&title).to_string();

        // Initialize Map Sources
        // Some(params)
        self.map_sources = MapSources::new(&self.params, &self.language_with_fallback())?;

        self.detect_region_zoom_globe();
        Ok(())
    }

    pub fn language_with_fallback(&self) -> String {
        const FALLBACK_LANGUAGE: &str = "en";
        let lang = self.lang.trim().to_ascii_lowercase();
        if lang.is_empty() {
            FALLBACK_LANGUAGE.to_string()
        } else {
            lang
        }
    }

    /// Detect region from parameters
    fn detect_region_zoom_globe(&mut self) {
        for v in self.map_sources.p.pieces() {
            if let Some(end) = v.strip_prefix("region:") {
                self.region_name = Self::process_region_name(end);
            } else if let Some(end) = v.strip_prefix("globe:") {
                self.globe = end.to_lowercase();
            } else if let Some(end) = v.strip_prefix("zoom:") {
                self.nlzoom = end.to_lowercase();
            }
        }
    }

    /// Build the output HTML
    pub fn build_output(&mut self) -> String {
        let lat = GeoParam::make_minsec(self.map_sources.p.latdeg());
        let lon = GeoParam::make_minsec(self.map_sources.p.londeg());

        // Build title
        let mytitle = if !self.title.is_empty() {
            format!("GeoHack - {}", self.title)
        } else if !self.pagename.is_empty() {
            format!("GeoHack - {}", self.pagename.replace('_', " "))
        } else {
            format!("GeoHack ({}; {})", lat.deg, lon.deg)
        };

        // Get logo URL
        let logo_url = self
            .logo_urls
            .get(&self.globe)
            .or_else(|| self.logo_urls.get(""))
            .unwrap_or(&String::new())
            .clone();

        // Build HTML output
        let mut html = String::new();
        html.push_str(&format!(r#"<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.0 Transitional//EN" "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd">
<html xmlns="http://www.w3.org/1999/xhtml"><head>
<title>{}</title>
<meta http-equiv="content-type" content="text/html; charset=utf-8" />
<meta name="viewport" content="width=device-width, initial-scale=1">
<meta name="robots" content="noindex" />
<link rel="shortcut icon" href="/geohack/siteicon.png" />
<link rel="stylesheet" type="text/css" media="screen" href="./main.css" />
<script type="text/javascript" src="//{}.wikipedia.org/w/index.php?title=MediaWiki:GeoHack.js&amp;action=raw&amp;ctype=text/javascript"></script>
</head>
<body class="mediawiki skin-modern">
<div id="mw_header"><h1 id="firstHeading">{}</h1></div>

    <div id="mw_main" style="margin-top:2em;">

<div id="mw_contentwrapper"><div id="mw_content">"#, mytitle, self.lang, mytitle));

        // Add zoom warning if needed
        if !self.nlzoom.is_empty() {
            html.push_str(
                r#"
<div class="mw-topboxes">
    <div class="zoom_error usermessage" style="background:#c00; color:white;">
Waarschuwing:
 op deze pagina word de verouderde parameter "zoom" gebruikt, in plaats
 daarvan zou "scale" of "dim" gebruikt moeten worden
     </div>
 </div>"#,
            );
        }

        // Add main content
        html.push_str(&self.page_content);

        html.push_str(
            r#"
</div></div>

<div id="mw_portlets">"#,
        );

        // Add logo
        html.push_str(&format!(
            r#"
<div class="portlet">
<div style="background:#000 url({}) center no-repeat; height:150px;"></div>
</div>"#,
            logo_url
        ));

        html.push_str(&self.actions);

        // Add languages section placeholder
        html.push_str(
            r#"
<!-- languages -->
    "#,
        );

        html.push_str(&self.languages);

        // Add footer
        html.push_str(r#"
<div class="portlet">
    <h5>GeoHack</h5>
    <div class="pBody">
        <ul>
            <li><a href="https://www.mediawiki.org/wiki/GeoHack">Documentation</a></li>
<!--            <li><a href="https://bitbucket.org/abbe98/geohack/issues">Bug tracker</a></li>
            <li><a href="https://bitbucket.org/magnusmanske/geohack">Source Code</a>)</li>-->
        </ul>
        <p style="text-align:center;"><a href="https://tools.wmflabs.org/"><img border="0" alt="Powered by Wikimedia Cloud Services" src="https://upload.wikimedia.org/wikipedia/commons/5/5a/Wikimedia_Cloud_Services_logo_with_text.svg" width="110" /></a></p>
    </div>
</div>
    <!-- actions -->
    "#);

        html.push_str(
            r#"
</div><!-- mw_portlets -->

</div>
</body>
</html>
"#,
        );

        html
    }

    /// Set page content from template
    pub fn set_page_content(&mut self, content: &str) {
        self.page_content = content.to_string();
        self.fix_wikipedia_html();
        self.map_sources.thetext = self.page_content.clone();
    }

    /// Process the template and build final output
    pub fn process(&mut self) -> String {
        // Build the map sources output
        let processed_content = self.map_sources.build_output(&self.pagename, &self.title);

        // Apply ugly hacks
        let processed_content = processed_content
            .replace("{nztmeasting}", "0")
            .replace("{nztmnorthing}", "0");

        // Handle localized services
        let mut final_content = processed_content.clone();
        if let Some(region) = &self.region_name {
            let locmaps =
                self.get_div_section(&processed_content, &format!("GEOTEMPLATE-{}", region), 0);
            let locinsert = self.get_div_section(&processed_content, "GEOTEMPLATE-LOCAL", 0);

            if !locmaps.is_empty() && !locinsert.is_empty() {
                final_content = final_content.replace(&locmaps, "");
                final_content = final_content.replace(&locinsert, &locmaps);
                let regions_div = self.get_div_section(&final_content, "GEOTEMPLATE-REGIONS", 0);
                final_content = final_content.replace(&regions_div, "");
            }
        }

        self.page_content = final_content;
        self.build_output()
    }

    fn fix_wikipedia_html(&mut self) {
        (self.page_content, self.actions, self.languages) = self.process_wikipedia_page();
    }

    // Main processing function
    fn process_wikipedia_page(&self) -> (String, String, String) {
        let mut page = self.page_content.clone();
        page = page
            .replace(
                r#" href="/w"#,
                &format!(r#" href="//{}.wikipedia.org/w"#, self.lang),
            )
            .replace(r#" role="navigation""#, "")
            .replace(r#" class="portlet""#, "");

        let actions_section = self.get_div_section(&page, "p-cactions", 0);
        let actions = actions_section.replace(r#"id="p-cactions""#, "");

        let lang_section = self.get_div_section(&page, "p-lang", 0);
        let languages = {
            lazy_static! {
                static ref RE: Regex =
                    Regex::new(r#" href="(https?:)//([a-z\-]+)?\.wikipedia\.org/wiki/[^"]*"#)
                        .unwrap();
            }

            let theparams_clone = self.params.to_string();
            let r_pagename_clone = self.pagename.to_string();

            RE.replace_all(&lang_section, |caps: &regex::Captures| {
                let lang_match = caps.get(2).map_or("", |m| m.as_str());
                format!(
                    r#" href="{}""#,
                    self.make_link(lang_match, &theparams_clone, &r_pagename_clone)
                )
            })
            .to_string()
        };

        // Remove edit links - loop until no more editsection spans are found
        loop {
            let original_page = page.clone();

            if let Some(pos) = page.find(r#"<span class="editsection""#) {
                let (before, after) = page.split_at(pos);
                let (before, after) = (before.to_string(), after.to_string());
                page = before.to_string();

                // Find the closing </span> tag
                if let Some(end_pos) = after.find("</span>") {
                    // Skip past the </span> tag
                    page.push_str(&after[end_pos + 7..]);
                } else {
                    // If no closing tag found, restore original and break
                    page = original_page;
                    break;
                }
            } else {
                // No more editsection spans found
                break;
            }

            // Check if page hasn't changed (equivalent to $op != $page)
            if original_page == page {
                break;
            }
        }

        // Build the page - extract content between markers
        if page.contains("<!-- start content -->") {
            // Split by start content marker and take the second part
            if let Some(pos) = page.find("<!-- start content -->") {
                page = page[pos + 22..].to_string(); // 22 is the length of "<!-- start content -->"

                // Split by end content marker and take the first part
                if let Some(end_pos) = page.find("<!-- end content -->") {
                    page = page[..end_pos].to_string();
                }
            }
        } else {
            // Alternative: use bodytext markers
            if let Some(pos) = page.find("<!-- bodytext -->") {
                page = page[pos + 17..].to_string(); // 17 is the length of "<!-- bodytext -->"

                if let Some(end_pos) = page.find("<!-- /bodytext -->") {
                    page = page[..end_pos].to_string();
                }
            }
        }

        (page, actions, languages)
    }

    fn process_region_name(end: &str) -> Option<String> {
        let mut region = end.to_uppercase();
        if let Some(pos) = region.find('-') {
            region = region[..pos].to_string();
        }
        if let Some(pos) = region.find('_') {
            region = region[..pos].to_string();
        }
        region = region.trim().to_string();
        if !region.is_empty() {
            Some(region)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fix_language_code() {
        let geohack = GeoHack::new().unwrap();
        assert_eq!(geohack.fix_language_code("en-US", "en"), "en-us");
        assert_eq!(geohack.fix_language_code("de", "en"), "de");
        assert_eq!(geohack.fix_language_code("123", "en"), "en");
    }

    #[test]
    fn test_make_link() {
        let geohack = GeoHack::new().unwrap();
        let link = geohack.make_link("en", "40.7_N_74.0_W", "Test_Page");
        assert_eq!(link, "/en/40.7_N_74.0_W?pagename=Test_Page");
    }

    #[test]
    fn test_logo_urls() {
        let geohack = GeoHack::new().unwrap();
        assert!(geohack.logo_urls.contains_key("earth"));
        assert!(geohack.logo_urls.contains_key("mars"));
        assert!(geohack.logo_urls.contains_key("moon"));
    }

    #[test]
    fn test_detect_region() {
        let mut geohack = GeoHack::new().unwrap();
        *geohack.map_sources.p.pieces_mut() =
            vec!["region:US-NY".to_string(), "globe:mars".to_string()];
        geohack.detect_region_zoom_globe();
        assert_eq!(geohack.region_name, Some("US".to_string()));
        assert_eq!(geohack.globe, "mars");
    }

    #[test]
    fn test_init_logo_urls() {
        let logo_urls = GeoHack::init_logo_urls();
        assert!(logo_urls.contains_key("earth"));
        assert!(logo_urls.contains_key("mars"));
        assert!(logo_urls.contains_key("moon"));
        assert_eq!(
            logo_urls.get("neptune").unwrap(),
            "//upload.wikimedia.org/wikipedia/commons/thumb/0/06/Neptune.jpg/150px-Neptune.jpg"
        );
    }
}
