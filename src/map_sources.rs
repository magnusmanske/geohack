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
use crate::traverse_mercator::{CH1903, OSGB36, TransverseMercator};
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
        let p = GeoParam::new(&params)?;

        Ok(MapSources {
            p,
            mapsources: "Map sources".to_string(),
            thetext: String::new(),
            params: Some(params.to_string()),
            language: language.to_string(),
        })
    }

    /*
    fn show() {
        global $wgOut;

        // No reason for robots to follow map links
        $wgOut->setRobotpolicy( 'noindex,nofollow' );

        $wgOut->setPagetitle( $this->mapsources );
        $wgOut->addWikiText( $this->build_output() );
    }
    */

    pub fn build_output(&mut self, r_pagename: &str, r_title: &str) -> String {
        // global $wgOut, $wgUser, $wgContLang, $wgRequest;
        /*
        if (($e = $this->p->get_error()) != "") {
            $wgOut->addHTML(
                   "<p>" . htmlspecialchars( $e ) . "</p>");
            $wgOut->output();
            wfErrorExit();
            return "";
        }
        */
        let mut attr = self.p.get_attr();

        // $sk = $wgUser->getSkin();

        //
        //   dim: to scale: convertion
        //
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

        //
        // zoom: comptability for nlwiki
        //
        if !attr.contains_key("scale") && attr.contains_key("zoom") {
            // Incompatible with {zoom} and {osmzoom}
            if let Some(zoom) = attr.get("zoom")
                && let Ok(zoom_val) = zoom.parse::<f64>()
            {
                let scale = 2_f64.powf(12.0 - zoom_val) * 100000.0;
                attr.insert("scale".to_string(), scale.to_string());
            }
        }

        //
        //  Default scale
        //
        let scale = attr
            .get("scale")
            .and_then(|s| s.parse::<i32>().ok())
            .unwrap_or(0);

        if scale <= 0 {
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

        let scale = attr
            .get("scale")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(300000.0);

        /*
         *  Convert coordinates to various Transverse Mercator forms
         */

        /* standard UTM */
        let mut utm = TransverseMercator::default();
        utm.lat_lon_to_utm(self.p.latdeg(), self.p.londeg());
        utm.zone = utm.lat_lon_to_utm_zone(self.p.latdeg(), self.p.londeg());

        /* fixed UTM as used by iNatur */
        let mut utm33 = TransverseMercator::default();
        utm33.lat_lon_zone_to_utm(self.p.latdeg(), self.p.londeg(), "33V");

        /*  UK National Grid, see http://www.gps.gov.uk/guide7.asp
         *  central meridian 47N 2W, offset 100km N 400km W */
        let mut osgb36 = OSGB36::default();
        let osgb36ref = osgb36.lat_lon_to_osgb36(self.p.latdeg(), self.p.londeg());

        /* Swiss traditional national grid */
        let mut ch1903 = CH1903::default();
        ch1903.lat_lon_to_ch1903(self.p.latdeg(), self.p.londeg());

        /*
         *  Mapquest style zoom
         *  9 is approx 1:6,000
         *  5 (default) is approx 1:333,000
         *  2 is approx 1:8,570,000
         *  0 is minimum
         */
        let zoom = if scale > 0.0 {
            (18.0 - scale.ln()) as i32
        } else {
            9
        }
        .clamp(0, 9);

        /*
         *  Openstreetmap style zoom
         *  18 (max) is 1:1,693
         *  n-1 is half of n
         *  2 (min) is about 1:111,000,000
         */
        let osmzoom = if scale > 0.0 {
            18 - ((scale.log2() - 1693_f64.log2()).round() as i32)
        } else {
            12
        }
        .clamp(0, 18);

        /*
         *  MSN uses an altitude equivalent
         *  instead of a scale:
         *  143 == 1:1000000 scale
         */
        let altitude = ((scale * 143.0 / 1000000.0) as i32).max(1);

        /*
         * Tiger and Google uses a span
         * FIXME calibration
         * 1.0 for 1:1000000
         */
        let span = scale * 1.0 / 1000000.0;

        /*
         * Multimap has a fixed set of scales
         * and will choke unless one of them are specified
         */
        let mmscale = if scale >= 30000000.0 {
            40000000
        } else if scale >= 14000000.0 {
            20000000
        } else if scale >= 6300000.0 {
            10000000
        } else if scale >= 2800000.0 {
            4000000
        } else if scale >= 1400000.0 {
            2000000
        } else if scale >= 700000.0 {
            1000000
        } else if scale >= 310000.0 {
            500000
        } else if scale >= 140000.0 {
            200000
        } else if scale >= 70000.0 {
            100000
        } else if scale >= 35000.0 {
            50000
        } else if scale >= 15000.0 {
            25000
        } else if scale >= 7000.0 {
            10000
        } else {
            5000
        };

        // Make minutes and seconds, and round
        let lat = GeoParam::make_minsec(self.p.latdeg());
        let lon = GeoParam::make_minsec(self.p.londeg());

        // Hack for negative, small degrees
        let latdegint = if self.p.latdeg() < 0.0 && lat.deg as i32 == 0 {
            "-0".to_string()
        } else {
            (lat.deg as i32).to_string()
        };

        let londegint = if self.p.londeg() < 0.0 && lon.deg as i32 == 0 {
            "-0".to_string()
        } else {
            (lon.deg as i32).to_string()
        };

        let latdeground = if self.p.latdeg() < 0.0 && lat.deg.round() as i32 == 0 {
            "-0".to_string()
        } else {
            lat.deg.round().to_string()
        };

        let londeground = if self.p.londeg() < 0.0 && lon.deg.round() as i32 == 0 {
            "-0".to_string()
        } else {
            lon.deg.round().to_string()
        };

        let latdeg_outer_abs = lat.deg.abs().ceil() as i32;
        let londeg_outer_abs = lon.deg.abs().ceil() as i32;

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

        let mut bstext = self.thetext.clone();

        /*
         * Replace in page
         */
        let search = vec![
            "{latdegdec}",
            "{londegdec}",
            "{latdegdecabs}",
            "{londegdecabs}",
            "{latdeground}",
            "{londeground}",
            "{latdegroundabs}",
            "{londegroundabs}",
            "{latdeg_outer_abs}",
            "{londeg_outer_abs}",
            "{latantipodes}",
            "{longantipodes}",
            "{londegneg}",
            "{latdegint}",
            "{londegint}",
            "{latdegabs}",
            "{londegabs}",
            "{latmindec}",
            "{lonmindec}",
            "{latminint}",
            "{lonminint}",
            "{latsecdec}",
            "{lonsecdec}",
            "{latsecint}",
            "{lonsecint}",
            "{latNS}",
            "{lonEW}",
            "{utmzone}",
            "{utmnorthing}",
            "{utmeasting}",
            "{utm33northing}",
            "{utm33easting}",
            "{osgb36ref}",
            "{osgb36northing}",
            "{osgb36easting}",
            "{ch1903northing}",
            "{ch1903easting}",
            "{scale}",
            "{mmscale}",
            "{altitude}",
            "{zoom}",
            "{osmzoom}",
            "{span}",
            "{type}",
            "{region}",
            "{globe}",
            "{page}",
            "{pagename}",
            "{title}",
            "{pagenamee}",
            "{titlee}",
            "{geocountry}",
            "{geoa1}",
            "{params}",
            "{language}",
            "{pagename_gmaps}",
        ];

        let longantipodes = if lon.deg > 0.0 {
            lon.deg - 180.0
        } else {
            lon.deg + 180.0
        };

        // Double encode for passthrew urls like toolserver.org/~para/cgi-bin/kmlexport
        let pagename_gmaps = urlencoding::encode(r_pagename)
            .into_owned()
            .replace("%20", "+");
        let pagename_gmaps = urlencoding::encode(&pagename_gmaps).into_owned();

        let replace = vec![
            lat.deg.to_string(),
            lon.deg.to_string(),
            lat.deg.abs().to_string(),
            lon.deg.abs().to_string(),
            latdeground,
            londeground,
            lat.deg.round().abs().to_string(),
            lon.deg.round().abs().to_string(),
            latdeg_outer_abs.to_string(),
            londeg_outer_abs.to_string(),
            (-lat.deg).to_string(),
            longantipodes.to_string(),
            (-lon.deg).to_string(),
            latdegint,
            londegint,
            (lat.deg.abs() as i32).to_string(),
            (lon.deg.abs() as i32).to_string(),
            lat.min.to_string(),
            lon.min.to_string(),
            (lat.min as i32).to_string(),
            (lon.min as i32).to_string(),
            lat.sec.to_string(),
            lon.sec.to_string(),
            (lat.sec as i32).to_string(),
            (lon.sec as i32).to_string(),
            lat.ns.clone(),
            lon.ew.clone(),
            utm.zone.clone(),
            utm.northing.round().to_string(),
            utm.easting.round().to_string(),
            utm33.northing.round().to_string(),
            utm33.easting.round().to_string(),
            osgb36ref,
            osgb36.northing.round().to_string(),
            osgb36.easting.round().to_string(),
            ch1903.northing.round().to_string(),
            ch1903.easting.round().to_string(),
            scale.to_string(),
            mmscale.to_string(),
            altitude.to_string(),
            zoom.to_string(),
            osmzoom.to_string(),
            span.to_string(),
            attr.get("type").unwrap_or(&String::new()).clone(),
            attr.get("region").unwrap_or(&String::new()).clone(),
            attr.get("globe").unwrap_or(&String::new()).clone(),
            attr.get("page").unwrap_or(&String::new()).clone(),
            r_pagename.to_string(),
            r_title.to_string(),
            urlencoding::encode(r_pagename).into_owned(),
            urlencoding::encode(r_title).into_owned(),
            region.clone(),
            attr.get("region")
                .map(|r| {
                    if r.len() >= 4 {
                        r[4..r.len().min(12)].to_uppercase()
                    } else {
                        String::new()
                    }
                })
                .unwrap_or_default(),
            html_escape::encode_text(&self.params.as_ref().unwrap_or(&"".to_string())).to_string(),
            html_escape::encode_text(&self.language).to_string(),
            pagename_gmaps,
        ];

        // Replace quoted versions first
        for (i, search_str) in search.iter().map(|s| Self::quote_html(s)).enumerate() {
            if let Some(replacement) = replace.get(i) {
                bstext = bstext.replace(&search_str, replacement);
            }
        }

        // Then replace unquoted versions
        for (i, search_str) in search.iter().enumerate() {
            if let Some(replacement) = replace.get(i) {
                bstext = bstext.replace(search_str, replacement);
            }
        }

        bstext
    }

    fn quote_html(s: &str) -> String {
        s.replace('{', "&#123;").replace('}', "&#125;")
    }
}

#[cfg(test)]
mod tests {
    // use super::*;

    use crate::map_sources::MapSources;

    #[test]
    fn test_html_escape() {
        assert_eq!(
            "&#123;pagename_gmaps&#125;",
            MapSources::quote_html("{pagename_gmaps}")
        );
    }
}
