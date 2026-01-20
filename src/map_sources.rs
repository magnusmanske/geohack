/** \file
 *
 *  Create a page which link to other map resources by adding the facility
 *  to embed coordinates in the URLs of these map resources according to
 *  various rules. See also
 *  http://en.wikipedia.org/wiki/Wikipedia:WikiProject_Geographical_coordinates
 *
 *  The displayed page is based on "Wikipedia:Map sources" (or similar)
 *
 *  \todo Translations
 *
 *  ----------------------------------------------------------------------
 *
 *  Copyright 2005, Egil Kvaleberg <egil@kvaleberg.no>
 *  Converted to Rust 2005 by <Magnus Manske> <magnusmanske@googlemail.com>
 *
 *  This program is free software; you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation; either version 2 of the License, or
 *  (at your option) any later version.
 *
 *  This program is distributed in the hope that it will be useful,
 *  but WITHOUT ANY WARRANTY; without even the implied warranty of
 *  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 *  GNU General Public License for more details.
 *
 *  You should have received a copy of the GNU General Public License
 *  along with this program; if not, write to the Free Software
 *  Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
 */
use crate::coordinate_group::CoordinateGroup;
use crate::geo_param::GeoParam;
use crate::misc_map_source_values::MiscMapSourceValues;
use crate::transverse_mercator_forms::TransverseMercatorForms;
use aho_corasick::AhoCorasick;
use anyhow::Result;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// Default scales for different location types
static DEFAULT_SCALES: Lazy<HashMap<&'static str, i32>> = Lazy::new(|| {
    HashMap::from([
        ("country", 10_000_000),    // 10 mill
        ("satellite", 10_000_000),  // 10 mill
        ("state", 3_000_000),       // 3 mill
        ("adm1st", 1_000_000),      // 1 mill
        ("adm2nd", 300_000),        // 300 thousand
        ("adm3rd", 100_000),        // 100 thousand
        ("city", 100_000),          // 100 thousand
        ("isle", 100_000),          // 100 thousand
        ("mountain", 100_000),      // 100 thousand
        ("river", 100_000),         // 100 thousand
        ("waterbody", 100_000),     // 100 thousand
        ("event", 50_000),          // 50 thousand
        ("forest", 50_000),         // 50 thousand
        ("glacier", 50_000),        // 50 thousand
        ("airport", 30_000),        // 30 thousand
        ("railwaystation", 10_000), // 10 thousand
        ("edu", 10_000),            // 10 thousand
        ("pass", 10_000),           // 10 thousand
        ("camera", 10_000),         // 10 thousand
        ("landmark", 10_000),       // 10 thousand
    ])
});

/// Multimap scale thresholds (threshold, result)
const MMSCALE_THRESHOLDS: &[(f64, i32)] = &[
    (30_000_000.0, 40_000_000),
    (14_000_000.0, 20_000_000),
    (6_300_000.0, 10_000_000),
    (2_800_000.0, 4_000_000),
    (1_400_000.0, 2_000_000),
    (700_000.0, 1_000_000),
    (310_000.0, 500_000),
    (140_000.0, 200_000),
    (70_000.0, 100_000),
    (35_000.0, 50_000),
    (15_000.0, 25_000),
    (7_000.0, 10_000),
];

#[derive(Debug, Clone, Default)]
pub struct MapSources {
    pub p: GeoParam,
    pub mapsources: String,
    pub thetext: String,
    pub params: Option<String>,
    pub language: String,
}

impl MapSources {
    pub fn new(params: &str, language: &str) -> Result<Self> {
        let p = GeoParam::new(params)?;

        Ok(MapSources {
            p,
            mapsources: "Map sources".to_string(),
            thetext: String::new(),
            params: Some(params.to_string()),
            language: language.to_string(),
        })
    }

    pub fn build_output(&mut self, r_pagename: &str, r_title: &str) -> Result<String> {
        let mut attr = self.p.get_attr();
        Self::scale_dim(&mut attr);
        Self::scale_zoom(&mut attr);
        self.default_scale(&mut attr);

        let tmf = TransverseMercatorForms::new(&self.p);
        let cg = CoordinateGroup::new(&self.p);
        let scale_float = Self::get_scale_float(&attr);
        let misc = MiscMapSourceValues {
            r_pagename: r_pagename.to_string(),
            r_title: r_title.to_string(),
            scale_float: Self::get_scale_float(&attr),
            zoom: Self::get_zoom(scale_float),
            osmzoom: Self::get_osmzoom(scale_float),
            altitude: Self::get_altitude(scale_float),
            span: Self::get_scale_float_span(scale_float),
            mmscale: Self::get_mmscale(scale_float),
            region: self.get_region(&attr),
            attr,
        };

        let ret = self.replace_in_page(tmf, cg, misc)?;

        Ok(ret)
    }

    fn replace_in_page(
        &mut self,
        tmf: TransverseMercatorForms,
        cg: CoordinateGroup,
        misc: MiscMapSourceValues,
    ) -> Result<String> {
        let pagename_gmaps = urlencoding::encode(&misc.r_pagename)
            .into_owned()
            .replace("%20", "+");
        let pagename_gmaps = urlencoding::encode(&pagename_gmaps).into_owned();

        let mut rep_map: HashMap<String, String> = HashMap::new();
        cg.add_rep_map(&mut rep_map);
        tmf.add_rep_map(&mut rep_map);
        misc.add_rep_map(&mut rep_map);
        rep_map.insert(
            "params".to_string(),
            html_escape::encode_text(self.params.as_deref().unwrap_or("")).to_string(),
        );
        rep_map.insert(
            "language".to_string(),
            html_escape::encode_text(&self.language).to_string(),
        );
        rep_map.insert("pagename_gmaps".to_string(), pagename_gmaps);

        // Build patterns and replacements for efficient multi-pattern replacement
        // We need to handle both {key} and &#123;key&#125; (HTML-escaped) formats
        let (patterns, replacements): (Vec<_>, Vec<_>) = rep_map
            .iter()
            .flat_map(|(key, value)| {
                [
                    (format!("{{{key}}}"), value.clone()),
                    (Self::quote_html(&format!("{{{key}}}")), value.clone()),
                ]
            })
            .unzip();

        // Use Aho-Corasick for efficient multi-pattern replacement
        let ac = AhoCorasick::new(&patterns)?;
        let result = ac.replace_all(&self.thetext, &replacements);

        Ok(result)
    }

    fn get_region(&mut self, attr: &HashMap<String, String>) -> String {
        /*
         *  Look up page from Wikipedia
         *  See if we have something in
         *  [[Wikipedia:Map sources]] or equivalent.
         *  A subpage can be specified
         */
        let mut src = self.mapsources.clone();
        let mut region = String::new();

        if let Some(page) = attr.get("page") {
            if !page.is_empty() {
                src.push('/');
                src.push_str(page); // subpage specified
            }
        } else if let Some(globe) = attr.get("globe") {
            if !globe.is_empty() {
                src.push('/');
                src.push_str(globe); // subpage specified
            }
        } else if let Some(reg) = attr.get("region")
            && !reg.is_empty()
        {
            region = format!("/{}", reg[..2.min(reg.len())].to_uppercase());
        }
        region
    }

    fn default_scale(&mut self, attr: &mut HashMap<String, String>) {
        // Default scale
        let scale_int = attr
            .get("scale")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);

        if scale_int > 0 {
            return;
        }

        let default = attr
            .get("default")
            .and_then(|d| d.parse::<i32>().ok())
            .filter(|&d| d > 0)
            .or_else(|| {
                // Look up default scale by type
                attr.get("type")
                    .and_then(|t| DEFAULT_SCALES.get(t.as_str()).copied())
                // FIXME: Scale according to city size, if available
            })
            .unwrap_or_else(|| {
                // No type and no default, make an assumption based on coordinate precision
                // FIXME: scale to input precision
                match self.p.coor().len() {
                    8 => 10_000,  // 10 thousand
                    6 => 100_000, // 100 thousand
                    _ => 300_000, // 300 thousand
                }
            });

        attr.insert("scale".to_string(), default.to_string());
    }

    fn quote_html(s: &str) -> String {
        s.replace('{', "&#123;").replace('}', "&#125;")
    }

    fn scale_dim(attr: &mut HashMap<String, String>) {
        //   dim: to scale: convertion
        if !attr.contains_key("scale") && attr.contains_key("dim") {
            // dia (m) [ (in per m) * (pixels per in) * screen size ]
            // Assume viewport size is 10 cm by 10 cm
            // FIXME document numbers
            // FIXME better convertion
            if let Some(dim) = attr.get("dim") {
                let dim_value = dim
                    .replace("km", "000")
                    .replace("m", "")
                    .parse::<f64>()
                    .unwrap_or(0.0);
                attr.insert("scale".to_string(), (dim_value / 0.1).to_string());
            }
        }
    }

    fn scale_zoom(attr: &mut HashMap<String, String>) {
        // zoom: comptability for nlwiki
        if !attr.contains_key("scale") && attr.contains_key("zoom") {
            // Incompatible with {zoom} and {osmzoom}
            if let Some(zoom) = attr.get("zoom")
                && let Ok(zoom_val) = zoom.parse::<f64>()
            {
                let scale = 2_f64.powf(12.0 - zoom_val) * 100000.0;
                attr.insert("scale".to_string(), scale.to_string());
            }
        }
    }

    fn get_mmscale(scale_float: f64) -> i32 {
        // Multimap has a fixed set of scales and will choke unless one of them is specified
        MMSCALE_THRESHOLDS
            .iter()
            .find(|(threshold, _)| scale_float >= *threshold)
            .map(|(_, scale)| *scale)
            .unwrap_or(5_000)
    }

    fn get_osmzoom(scale_float: f64) -> i32 {
        /*
         *  Openstreetmap style zoom
         *  18 (max) is 1:1,693
         *  n-1 is half of n
         *  2 (min) is about 1:111,000,000
         */
        if scale_float > 0.0 {
            18 - ((scale_float.log2() - 1693_f64.log2()).round() as i32)
        } else {
            12
        }
        .clamp(0, 18)
    }

    fn get_zoom(scale_float: f64) -> i32 {
        /*
         *  Mapquest style zoom
         *  9 is approx 1:6,000
         *  5 (default) is approx 1:333,000
         *  2 is approx 1:8,570,000
         *  0 is minimum
         */
        if scale_float > 0.0 {
            (18.0 - scale_float.ln()) as i32
        } else {
            9
        }
        .clamp(0, 9)
    }

    fn get_altitude(scale_float: f64) -> i32 {
        /*
         *  MSN uses an altitude equivalent
         *  instead of a scale:
         *  143 == 1:1000000 scale
         */
        ((scale_float * 143.0 / 1000000.0) as i32).max(1)
    }

    fn get_scale_float_span(scale_float: f64) -> f64 {
        /*
         * Tiger and Google uses a span
         * FIXME calibration
         * 1.0 for 1:1000000
         */
        scale_float * 1.0 / 1000000.0
    }

    fn get_scale_float(attr: &HashMap<String, String>) -> f64 {
        attr.get("scale")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(300000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_html_escape() {
        assert_eq!(
            "&#123;pagename_gmaps&#125;",
            MapSources::quote_html("{pagename_gmaps}")
        );
    }

    #[test]
    fn test_get_mmscale_thresholds() {
        // Test various scale values to ensure they map to correct multimap scales
        assert_eq!(MapSources::get_mmscale(50_000_000.0), 40_000_000);
        assert_eq!(MapSources::get_mmscale(30_000_000.0), 40_000_000);
        assert_eq!(MapSources::get_mmscale(20_000_000.0), 20_000_000);
        assert_eq!(MapSources::get_mmscale(10_000_000.0), 10_000_000);
        assert_eq!(MapSources::get_mmscale(1_000_000.0), 1_000_000);
        assert_eq!(MapSources::get_mmscale(100_000.0), 100_000);
        assert_eq!(MapSources::get_mmscale(50_000.0), 50_000);
        assert_eq!(MapSources::get_mmscale(5_000.0), 5_000);
        assert_eq!(MapSources::get_mmscale(1_000.0), 5_000);
    }

    #[test]
    fn test_get_osmzoom() {
        // Test OSM zoom levels
        assert_eq!(MapSources::get_osmzoom(0.0), 12); // Default for 0
        assert!(MapSources::get_osmzoom(1_000.0) >= 15); // High zoom for small scale
        assert!(MapSources::get_osmzoom(10_000_000.0) <= 5); // Low zoom for large scale
        // Ensure clamping works
        assert!(MapSources::get_osmzoom(1.0) <= 18);
        assert!(MapSources::get_osmzoom(1_000_000_000.0) >= 0);
    }

    #[test]
    fn test_get_zoom() {
        // Test Mapquest-style zoom
        assert_eq!(MapSources::get_zoom(0.0), 9); // Default for 0
        assert!(MapSources::get_zoom(100_000.0) >= 0);
        assert!(MapSources::get_zoom(100_000.0) <= 9);
    }

    #[test]
    fn test_get_altitude() {
        // Test altitude calculation
        assert_eq!(MapSources::get_altitude(1_000_000.0), 143);
        assert_eq!(MapSources::get_altitude(500_000.0), 71);
        assert_eq!(MapSources::get_altitude(100.0), 1); // Minimum is 1
    }

    #[test]
    fn test_get_scale_float_span() {
        assert_eq!(MapSources::get_scale_float_span(1_000_000.0), 1.0);
        assert_eq!(MapSources::get_scale_float_span(500_000.0), 0.5);
        assert_eq!(MapSources::get_scale_float_span(2_000_000.0), 2.0);
    }

    #[test]
    fn test_get_scale_float() {
        let mut attr = HashMap::new();
        attr.insert("scale".to_string(), "500000".to_string());
        assert_eq!(MapSources::get_scale_float(&attr), 500_000.0);

        // Default when no scale
        let empty_attr = HashMap::new();
        assert_eq!(MapSources::get_scale_float(&empty_attr), 300_000.0);

        // Invalid scale value
        let mut invalid_attr = HashMap::new();
        invalid_attr.insert("scale".to_string(), "invalid".to_string());
        assert_eq!(MapSources::get_scale_float(&invalid_attr), 300_000.0);
    }

    #[test]
    fn test_map_sources_new() {
        let ms = MapSources::new("40_N_74_W_type:city", "en").unwrap();
        assert_eq!(ms.language, "en");
        assert!(ms.params.is_some());
        assert_eq!(ms.mapsources, "Map sources");
    }

    #[test]
    fn test_scale_dim_conversion() {
        let mut attr = HashMap::new();
        attr.insert("dim".to_string(), "1000".to_string());

        MapSources::scale_dim(&mut attr);

        // dim 1000m should convert to scale 10000 (1000 / 0.1)
        assert!(attr.contains_key("scale"));
        let scale: f64 = attr.get("scale").unwrap().parse().unwrap();
        assert_eq!(scale, 10_000.0);
    }

    #[test]
    fn test_scale_dim_with_km() {
        let mut attr = HashMap::new();
        attr.insert("dim".to_string(), "1km".to_string());

        MapSources::scale_dim(&mut attr);

        // dim 1km = 1000m should convert to scale 10000
        let scale: f64 = attr.get("scale").unwrap().parse().unwrap();
        assert_eq!(scale, 10_000.0);
    }

    #[test]
    fn test_scale_zoom_conversion() {
        let mut attr = HashMap::new();
        attr.insert("zoom".to_string(), "12".to_string());

        MapSources::scale_zoom(&mut attr);

        assert!(attr.contains_key("scale"));
    }

    #[test]
    fn test_default_scale_by_type() {
        let mut ms = MapSources::new("40_N_74_W", "en").unwrap();
        let mut attr = HashMap::new();
        attr.insert("type".to_string(), "country".to_string());

        ms.default_scale(&mut attr);

        let scale: i32 = attr.get("scale").unwrap().parse().unwrap();
        assert_eq!(scale, 10_000_000);
    }

    #[test]
    fn test_default_scale_by_coordinate_precision() {
        let mut ms = MapSources::new("40_30_45_N_74_0_21_W", "en").unwrap();
        let mut attr = HashMap::new();

        ms.default_scale(&mut attr);

        // 8-piece coordinates should get 10000 scale
        let scale: i32 = attr.get("scale").unwrap().parse().unwrap();
        assert_eq!(scale, 10_000);
    }
}
