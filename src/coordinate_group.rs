use crate::geo_param::{GeoParam, MinSecResult};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct CoordinateGroup {
    lat: MinSecResult,
    lon: MinSecResult,
    latdegint: String,
    londegint: String,
    latdeground: String,
    londeground: String,
    latdeg_outer_abs: i32,
    londeg_outer_abs: i32,
    longantipodes: f64,
}

impl CoordinateGroup {
    pub fn new(p: &GeoParam) -> Self {
        // Make minutes and seconds, and round
        let lat = GeoParam::make_minsec(p.latdeg());
        let lon = GeoParam::make_minsec(p.londeg());

        // Hack for negative, small degrees
        let latdegint = if p.latdeg() < 0.0 && lat.deg as i32 == 0 {
            "-0".to_string()
        } else {
            (lat.deg as i32).to_string()
        };

        let londegint = if p.londeg() < 0.0 && lon.deg as i32 == 0 {
            "-0".to_string()
        } else {
            (lon.deg as i32).to_string()
        };

        let latdeground = if p.latdeg() < 0.0 && lat.deg.round() as i32 == 0 {
            "-0".to_string()
        } else {
            lat.deg.round().to_string()
        };

        let londeground = if p.londeg() < 0.0 && lon.deg.round() as i32 == 0 {
            "-0".to_string()
        } else {
            lon.deg.round().to_string()
        };

        let latdeg_outer_abs = lat.deg.abs().ceil() as i32;
        let londeg_outer_abs = lon.deg.abs().ceil() as i32;

        let longantipodes = if lon.deg > 0.0 {
            lon.deg - 180.0
        } else {
            lon.deg + 180.0
        };
        Self {
            lat,
            lon,
            latdegint,
            londegint,
            latdeground,
            londeground,
            latdeg_outer_abs,
            londeg_outer_abs,
            longantipodes,
        }
    }

    pub fn add_rep_map(&self, rep_map: &mut HashMap<String, String>) {
        rep_map.insert("latdegdec".to_string(), self.lat.deg.to_string());
        rep_map.insert("londegdec".to_string(), self.lon.deg.to_string());
        rep_map.insert("latdegdecabs".to_string(), self.lat.deg.abs().to_string());
        rep_map.insert("londegdecabs".to_string(), self.lon.deg.abs().to_string());
        rep_map.insert("latdeground".to_string(), self.latdeground.to_string());
        rep_map.insert("londeground".to_string(), self.londeground.to_string());
        rep_map.insert(
            "latdegroundabs".to_string(),
            self.lat.deg.round().abs().to_string(),
        );
        rep_map.insert(
            "londegroundabs".to_string(),
            self.lon.deg.round().abs().to_string(),
        );
        rep_map.insert(
            "latdeg_outer_abs".to_string(),
            self.latdeg_outer_abs.to_string(),
        );
        rep_map.insert(
            "londeg_outer_abs".to_string(),
            self.londeg_outer_abs.to_string(),
        );
        rep_map.insert("latantipodes".to_string(), (-self.lat.deg).to_string());
        rep_map.insert("longantipodes".to_string(), self.longantipodes.to_string());
        rep_map.insert("londegneg".to_string(), (-self.lon.deg).to_string());
        rep_map.insert("latdegint".to_string(), self.latdegint.to_string());
        rep_map.insert("londegint".to_string(), self.londegint.to_string());
        rep_map.insert(
            "latdegabs".to_string(),
            (self.lat.deg.abs() as i32).to_string(),
        );
        rep_map.insert(
            "londegabs".to_string(),
            (self.lon.deg.abs() as i32).to_string(),
        );
        rep_map.insert("latmindec".to_string(), self.lat.min.to_string());
        rep_map.insert("lonmindec".to_string(), self.lon.min.to_string());
        rep_map.insert("latminint".to_string(), (self.lat.min as i32).to_string());
        rep_map.insert("lonminint".to_string(), (self.lon.min as i32).to_string());
        rep_map.insert("latsecdec".to_string(), self.lat.sec.to_string());
        rep_map.insert("lonsecdec".to_string(), self.lon.sec.to_string());
        rep_map.insert("latsecint".to_string(), (self.lat.sec as i32).to_string());
        rep_map.insert("lonsecint".to_string(), (self.lon.sec as i32).to_string());
        rep_map.insert("latNS".to_string(), self.lat.ns.clone());
        rep_map.insert("lonEW".to_string(), self.lon.ew.clone());
    }
}
