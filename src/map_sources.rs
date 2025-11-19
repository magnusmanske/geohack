use crate::coordinate_group::CoordinateGroup;
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
use crate::geo_param::GeoParam;
use crate::misc_map_source_values::MiscMapSourceValues;
use crate::transverse_mercator_forms::TransverseMercatorForms;
use anyhow::Result;
use std::collections::HashMap;

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
        let mut ret = self.thetext.clone();
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
            html_escape::encode_text(&self.params.as_ref().unwrap_or(&"".to_string())).to_string(),
        );
        rep_map.insert(
            "language".to_string(),
            html_escape::encode_text(&self.language).to_string(),
        );
        rep_map.insert("pagename_gmaps".to_string(), pagename_gmaps);

        for (search_str, replacement) in rep_map
            .iter()
            .map(|(s, r)| (Self::quote_html(&format!("{{{s}}}")), r))
        {
            ret = ret.replace(&search_str, replacement);
        }
        for (search_str, replacement) in rep_map.iter().map(|(s, r)| (format!("{{{s}}}"), r)) {
            ret = ret.replace(&search_str, replacement);
        }
        Ok(ret)
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
        //  Default scale
        let scale_int = attr
            .get("scale")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);

        if scale_int <= 0 {
            let mut default = attr
                .get("default")
                .and_then(|d| d.parse::<i32>().ok())
                .unwrap_or(0);

            if default == 0 {
                let default_scale: HashMap<&str, i32> = [
                    ("country", 10000000),     // 10 mill
                    ("satellite", 10000000),   // 10 mill
                    ("state", 3000000),        // 3 mill
                    ("adm1st", 1000000),       // 1 mill
                    ("adm2nd", 300000),        // 300 thousand
                    ("adm3rd", 100000),        // 100 thousand
                    ("city", 100000),          // 100 thousand
                    ("isle", 100000),          // 100 thousand
                    ("mountain", 100000),      // 100 thousand
                    ("river", 100000),         // 100 thousand
                    ("waterbody", 100000),     // 100 thousand
                    ("event", 50000),          // 50 thousand
                    ("forest", 50000),         // 50 thousand
                    ("glacier", 50000),        // 50 thousand
                    ("airport", 30000),        // 30 thousand
                    ("railwaystation", 10000), // 10 thousand
                    ("edu", 10000),            // 10 thousand
                    ("pass", 10000),           // 10 thousand
                    ("camera", 10000),         // 10 thousand
                    ("landmark", 10000),       // 10 thousand
                ]
                .iter()
                .cloned()
                .collect();

                if let Some(type_attr) = attr.get("type")
                    && let Some(&scale_val) = default_scale.get(type_attr.as_str())
                {
                    default = scale_val;
                }

                // FIXME: Scale according to city size, if available
            }

            if default == 0 {
                /* No type and no default, make an assumption */
                // FIXME: scale to input precision
                default = if self.p.coor().len() == 8 {
                    10000 // 10 thousand
                } else if self.p.coor().len() == 6 {
                    100000 // 500 thousand
                } else {
                    300000 // 3000 thousand
                };
            }
            attr.insert("scale".to_string(), default.to_string());
        }
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
        /*
         * Multimap has a fixed set of scales
         * and will choke unless one of them are specified
         */
        if scale_float >= 30000000.0 {
            40000000
        } else if scale_float >= 14000000.0 {
            20000000
        } else if scale_float >= 6300000.0 {
            10000000
        } else if scale_float >= 2800000.0 {
            4000000
        } else if scale_float >= 1400000.0 {
            2000000
        } else if scale_float >= 700000.0 {
            1000000
        } else if scale_float >= 310000.0 {
            500000
        } else if scale_float >= 140000.0 {
            200000
        } else if scale_float >= 70000.0 {
            100000
        } else if scale_float >= 35000.0 {
            50000
        } else if scale_float >= 15000.0 {
            25000
        } else if scale_float >= 7000.0 {
            10000
        } else {
            5000
        }
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
}
