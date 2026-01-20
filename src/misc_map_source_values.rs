use std::collections::HashMap;

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
}
