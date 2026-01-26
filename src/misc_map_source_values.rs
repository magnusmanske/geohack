use std::collections::HashMap;

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
pub struct MiscMapSourceValues {
    pub r_pagename: String,
    pub r_title: String,
    pub scale_float: f64,
    pub zoom: i32,
    pub osmzoom: i32,
    pub altitude: i32,
    pub span: f64,
    pub mmscale: i32,
    pub region: String,
    pub attr: HashMap<String, String>,
}

impl MiscMapSourceValues {
    /// Create a new MiscMapSourceValues with computed derived values
    pub fn new(
        r_pagename: &str,
        r_title: &str,
        region: &str,
        attr: HashMap<String, String>,
    ) -> Self {
        let scale_float = Self::get_scale_float(&attr);
        Self {
            r_pagename: r_pagename.to_string(),
            r_title: r_title.to_string(),
            scale_float,
            zoom: Self::get_zoom(scale_float),
            osmzoom: Self::get_osmzoom(scale_float),
            altitude: Self::get_altitude(scale_float),
            span: Self::get_scale_float_span(scale_float),
            mmscale: Self::get_mmscale(scale_float),
            region: region.to_string(),
            attr,
        }
    }

    pub fn get_scale_float(attr: &HashMap<String, String>) -> f64 {
        attr.get("scale")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(300000.0)
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

    pub fn region_string(&self) -> String {
        self.attr
            .get("region")
            .map(|r| {
                if r.len() >= 4 {
                    r[4..r.len().min(12)].to_uppercase()
                } else {
                    String::new()
                }
            })
            .unwrap_or_default()
    }

    pub fn add_rep_map(&self, rep_map: &mut HashMap<String, String>) {
        let empty = String::new();
        insert_map!(rep_map, {
            "scale" => self.scale_float,
            "mmscale" => self.mmscale,
            "altitude" => self.altitude,
            "zoom" => self.zoom,
            "osmzoom" => self.osmzoom,
            "span" => self.span,
            "type" => self.attr.get("type").unwrap_or(&empty),
            "region" => self.attr.get("region").unwrap_or(&empty),
            "globe" => self.attr.get("globe").unwrap_or(&empty),
            "page" => self.attr.get("page").unwrap_or(&empty),
            "pagename" => &self.r_pagename,
            "title" => &self.r_title,
            "pagenamee" => urlencoding::encode(&self.r_pagename),
            "titlee" => urlencoding::encode(&self.r_title),
            "geocountry" => &self.region,
            "geoa1" => self.region_string(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_region_string_with_long_region() {
        let mut attr = HashMap::new();
        // "US-NY-NYC" has length 9, so characters 4..9 = "Y-NYC"
        attr.insert("region".to_string(), "US-NY-NYC".to_string());

        let msv = MiscMapSourceValues {
            attr,
            ..Default::default()
        };

        // Should extract characters from index 4 onwards (Y-NYC) and uppercase
        assert_eq!(msv.region_string(), "Y-NYC");
    }

    #[test]
    fn test_region_string_with_short_region() {
        let mut attr = HashMap::new();
        attr.insert("region".to_string(), "US".to_string());

        let msv = MiscMapSourceValues {
            attr,
            ..Default::default()
        };

        // Region too short (< 4 chars), should return empty
        assert_eq!(msv.region_string(), "");
    }

    #[test]
    fn test_region_string_no_region() {
        let msv = MiscMapSourceValues::default();

        // No region attribute, should return empty
        assert_eq!(msv.region_string(), "");
    }

    #[test]
    fn test_add_rep_map() {
        let mut attr = HashMap::new();
        attr.insert("type".to_string(), "city".to_string());
        attr.insert("region".to_string(), "US-NY".to_string());

        let msv = MiscMapSourceValues {
            r_pagename: "Test Page".to_string(),
            r_title: "Test Title".to_string(),
            scale_float: 100000.0,
            zoom: 5,
            osmzoom: 12,
            altitude: 14,
            span: 0.1,
            mmscale: 100000,
            region: "/US".to_string(),
            attr,
        };

        let mut rep_map = HashMap::new();
        msv.add_rep_map(&mut rep_map);

        assert_eq!(rep_map.get("pagename").unwrap(), "Test Page");
        assert_eq!(rep_map.get("title").unwrap(), "Test Title");
        assert_eq!(rep_map.get("scale").unwrap(), "100000");
        assert_eq!(rep_map.get("type").unwrap(), "city");
        assert_eq!(rep_map.get("region").unwrap(), "US-NY");
        assert_eq!(rep_map.get("geocountry").unwrap(), "/US");

        // URL-encoded versions
        assert_eq!(rep_map.get("pagenamee").unwrap(), "Test%20Page");
        assert_eq!(rep_map.get("titlee").unwrap(), "Test%20Title");
    }

    #[test]
    fn test_add_rep_map_empty_attrs() {
        let msv = MiscMapSourceValues::default();

        let mut rep_map = HashMap::new();
        msv.add_rep_map(&mut rep_map);

        // Empty attrs should produce empty strings
        assert_eq!(rep_map.get("type").unwrap(), "");
        assert_eq!(rep_map.get("region").unwrap(), "");
        assert_eq!(rep_map.get("globe").unwrap(), "");
    }

    #[test]
    fn test_get_mmscale_thresholds() {
        // Test various scale values to ensure they map to correct multimap scales
        assert_eq!(MiscMapSourceValues::get_mmscale(50_000_000.0), 40_000_000);
        assert_eq!(MiscMapSourceValues::get_mmscale(30_000_000.0), 40_000_000);
        assert_eq!(MiscMapSourceValues::get_mmscale(20_000_000.0), 20_000_000);
        assert_eq!(MiscMapSourceValues::get_mmscale(10_000_000.0), 10_000_000);
        assert_eq!(MiscMapSourceValues::get_mmscale(1_000_000.0), 1_000_000);
        assert_eq!(MiscMapSourceValues::get_mmscale(100_000.0), 100_000);
        assert_eq!(MiscMapSourceValues::get_mmscale(50_000.0), 50_000);
        assert_eq!(MiscMapSourceValues::get_mmscale(5_000.0), 5_000);
        assert_eq!(MiscMapSourceValues::get_mmscale(1_000.0), 5_000);
    }

    #[test]
    fn test_get_osmzoom() {
        // Test OSM zoom levels
        assert_eq!(MiscMapSourceValues::get_osmzoom(0.0), 12); // Default for 0
        assert!(MiscMapSourceValues::get_osmzoom(1_000.0) >= 15); // High zoom for small scale
        assert!(MiscMapSourceValues::get_osmzoom(10_000_000.0) <= 5); // Low zoom for large scale
        // Ensure clamping works
        assert!(MiscMapSourceValues::get_osmzoom(1.0) <= 18);
        assert!(MiscMapSourceValues::get_osmzoom(1_000_000_000.0) >= 0);
    }

    #[test]
    fn test_get_zoom() {
        // Test Mapquest-style zoom
        assert_eq!(MiscMapSourceValues::get_zoom(0.0), 9); // Default for 0
        assert!(MiscMapSourceValues::get_zoom(100_000.0) >= 0);
        assert!(MiscMapSourceValues::get_zoom(100_000.0) <= 9);
    }

    #[test]
    fn test_get_altitude() {
        // Test altitude calculation
        assert_eq!(MiscMapSourceValues::get_altitude(1_000_000.0), 143);
        assert_eq!(MiscMapSourceValues::get_altitude(500_000.0), 71);
        assert_eq!(MiscMapSourceValues::get_altitude(100.0), 1); // Minimum is 1
    }

    #[test]
    fn test_get_scale_float_span() {
        assert_eq!(MiscMapSourceValues::get_scale_float_span(1_000_000.0), 1.0);
        assert_eq!(MiscMapSourceValues::get_scale_float_span(500_000.0), 0.5);
        assert_eq!(MiscMapSourceValues::get_scale_float_span(2_000_000.0), 2.0);
    }

    #[test]
    fn test_get_scale_float() {
        let mut attr = HashMap::new();
        attr.insert("scale".to_string(), "500000".to_string());
        assert_eq!(MiscMapSourceValues::get_scale_float(&attr), 500_000.0);

        // Default when no scale
        let empty_attr = HashMap::new();
        assert_eq!(MiscMapSourceValues::get_scale_float(&empty_attr), 300_000.0);

        // Invalid scale value
        let mut invalid_attr = HashMap::new();
        invalid_attr.insert("scale".to_string(), "invalid".to_string());
        assert_eq!(
            MiscMapSourceValues::get_scale_float(&invalid_attr),
            300_000.0
        );
    }

    #[test]
    fn test_new_computes_derived_values() {
        let mut attr = HashMap::new();
        attr.insert("scale".to_string(), "1000000".to_string());

        let msv = MiscMapSourceValues::new("Test Page", "Test Title", "/US", attr);

        assert_eq!(msv.scale_float, 1_000_000.0);
        assert_eq!(msv.altitude, 143); // 1000000 * 143 / 1000000
        assert_eq!(msv.span, 1.0); // 1000000 / 1000000
        assert_eq!(msv.mmscale, 1_000_000);
        assert_eq!(msv.r_pagename, "Test Page");
        assert_eq!(msv.r_title, "Test Title");
        assert_eq!(msv.region, "/US");
    }
}
