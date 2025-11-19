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
        rep_map.insert("scale".to_string(), self.scale_float.to_string());
        rep_map.insert("mmscale".to_string(), self.mmscale.to_string());
        rep_map.insert("altitude".to_string(), self.altitude.to_string());
        rep_map.insert("zoom".to_string(), self.zoom.to_string());
        rep_map.insert("osmzoom".to_string(), self.osmzoom.to_string());
        rep_map.insert("span".to_string(), self.span.to_string());
        rep_map.insert(
            "type".to_string(),
            self.attr.get("type").unwrap_or(&String::new()).clone(),
        );
        rep_map.insert(
            "region".to_string(),
            self.attr.get("region").unwrap_or(&String::new()).clone(),
        );
        rep_map.insert(
            "globe".to_string(),
            self.attr.get("globe").unwrap_or(&String::new()).clone(),
        );
        rep_map.insert(
            "page".to_string(),
            self.attr.get("page").unwrap_or(&String::new()).clone(),
        );
        rep_map.insert("pagename".to_string(), self.r_pagename.to_string());
        rep_map.insert("title".to_string(), self.r_title.to_string());
        rep_map.insert(
            "pagenamee".to_string(),
            urlencoding::encode(&self.r_pagename).into_owned(),
        );
        rep_map.insert(
            "titlee".to_string(),
            urlencoding::encode(&self.r_title).into_owned(),
        );
        rep_map.insert("geocountry".to_string(), self.region.clone());
        rep_map.insert("geoa1".to_string(), self.region_string());
    }
}
