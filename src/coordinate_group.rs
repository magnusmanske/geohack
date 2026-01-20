use crate::geo_param::{GeoParam, MinSecResult};
use crate::insert_map;
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
        insert_map!(rep_map, {
            "latdegdec" => self.lat.deg,
            "londegdec" => self.lon.deg,
            "latdegdecabs" => self.lat.deg.abs(),
            "londegdecabs" => self.lon.deg.abs(),
            "latdeground" => &self.latdeground,
            "londeground" => &self.londeground,
            "latdegroundabs" => self.lat.deg.round().abs(),
            "londegroundabs" => self.lon.deg.round().abs(),
            "latdeg_outer_abs" => self.latdeg_outer_abs,
            "londeg_outer_abs" => self.londeg_outer_abs,
            "latantipodes" => -self.lat.deg,
            "longantipodes" => self.longantipodes,
            "londegneg" => -self.lon.deg,
            "latdegint" => &self.latdegint,
            "londegint" => &self.londegint,
            "latdegabs" => self.lat.deg.abs() as i32,
            "londegabs" => self.lon.deg.abs() as i32,
            "latmindec" => self.lat.min,
            "lonmindec" => self.lon.min,
            "latminint" => self.lat.min as i32,
            "lonminint" => self.lon.min as i32,
            "latsecdec" => self.lat.sec,
            "lonsecdec" => self.lon.sec,
            "latsecint" => self.lat.sec as i32,
            "lonsecint" => self.lon.sec as i32,
            "latNS" => &self.lat.ns,
            "lonEW" => &self.lon.ew,
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coordinate_group_positive() {
        let geo = GeoParam::new("40.7128_N_74.0060_W").unwrap();
        let cg = CoordinateGroup::new(&geo);

        assert_eq!(cg.latdegint, "40");
        assert_eq!(cg.londegint, "-74");
        assert!(cg.lat.deg > 40.0);
        assert!(cg.lon.deg < -73.0);
    }

    #[test]
    fn test_coordinate_group_negative_zero() {
        // Test the negative zero case (small negative values that round to 0)
        let geo = GeoParam::new("-0.3_N_-0.3_E").unwrap();
        let cg = CoordinateGroup::new(&geo);

        assert_eq!(cg.latdegint, "-0");
        assert_eq!(cg.londegint, "-0");
    }

    #[test]
    fn test_coordinate_group_antipodes() {
        let geo = GeoParam::new("45_N_90_E").unwrap();
        let cg = CoordinateGroup::new(&geo);

        // Antipodes: 90 - 180 = -90
        assert_eq!(cg.longantipodes, -90.0);

        let geo2 = GeoParam::new("45_N_90_W").unwrap();
        let cg2 = CoordinateGroup::new(&geo2);

        // Antipodes: -90 + 180 = 90
        assert_eq!(cg2.longantipodes, 90.0);
    }

    #[test]
    fn test_coordinate_group_rep_map() {
        let geo = GeoParam::new("51_30_28_N_0_07_41_W").unwrap();
        let cg = CoordinateGroup::new(&geo);

        let mut rep_map = HashMap::new();
        cg.add_rep_map(&mut rep_map);

        // Check that all expected keys are present
        assert!(rep_map.contains_key("latdegdec"));
        assert!(rep_map.contains_key("londegdec"));
        assert!(rep_map.contains_key("latNS"));
        assert!(rep_map.contains_key("lonEW"));
        assert!(rep_map.contains_key("latantipodes"));
        assert!(rep_map.contains_key("longantipodes"));

        // Check direction
        assert_eq!(rep_map.get("latNS").unwrap(), "N");
        assert_eq!(rep_map.get("lonEW").unwrap(), "W");
    }

    #[test]
    fn test_coordinate_group_outer_abs() {
        let geo = GeoParam::new("40.3_N_74.7_W").unwrap();
        let cg = CoordinateGroup::new(&geo);

        // ceil(40.3) = 41, ceil(74.7) = 75
        assert_eq!(cg.latdeg_outer_abs, 41);
        assert_eq!(cg.londeg_outer_abs, 75);
    }
}
