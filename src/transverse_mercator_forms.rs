use crate::geo_param::GeoParam;
use crate::traverse_mercator::{CH1903, OSGB36, Utm};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct TransverseMercatorForms {
    utm: Utm,
    utm33: Utm,
    osgb36: OSGB36,
    ch1903: CH1903,
}

impl TransverseMercatorForms {
    /// Convert coordinates to various Transverse Mercator forms
    pub fn new(p: &GeoParam) -> Self {
        Self {
            /* standard UTM */
            utm: Utm::from_lat_lon(p.latdeg(), p.londeg()),
            /* fixed UTM zone as used by iNatur */
            utm33: Utm::from_lat_lon_forced_zone(p.latdeg(), p.londeg(), 33),
            /* UK National Grid */
            osgb36: OSGB36::from_lat_lon(p.latdeg(), p.londeg()),
            /* Swiss traditional national grid */
            ch1903: CH1903::from_lat_lon(p.latdeg(), p.londeg()),
        }
    }

    pub fn add_rep_map(&self, rep_map: &mut HashMap<String, String>) {
        insert_map!(rep_map, {
            "utmzone" => self.utm.zone(),
            "utmnorthing" => self.utm.northing().round(),
            "utmeasting" => self.utm.easting().round(),
            "utm33northing" => self.utm33.northing().round(),
            "utm33easting" => self.utm33.easting().round(),
            "osgb36ref" => self.osgb36.grid_reference(),
            "osgb36northing" => self.osgb36.northing().round(),
            "osgb36easting" => self.osgb36.easting().round(),
            "ch1903northing" => self.ch1903.northing().round(),
            "ch1903easting" => self.ch1903.easting().round(),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transverse_mercator_forms_london() {
        // London coordinates
        let geo = GeoParam::new("51.5074_N_0.1278_W").unwrap();
        let tmf = TransverseMercatorForms::new(&geo);

        // UTM zone should be 30 or 31 for London
        assert!(tmf.utm.zone().starts_with("30") || tmf.utm.zone().starts_with("31"));

        // OSGB36 should produce valid reference for London
        assert!(!tmf.osgb36.grid_reference().is_empty());
        assert!(tmf.osgb36.grid_reference().starts_with('T')); // London is in TQ grid square
    }

    #[test]
    fn test_transverse_mercator_forms_swiss() {
        // Bern, Switzerland
        let geo = GeoParam::new("46.9480_N_7.4474_E").unwrap();
        let tmf = TransverseMercatorForms::new(&geo);

        // CH1903 should produce valid coordinates for Switzerland
        assert!(tmf.ch1903.northing() > 100_000.0);
        assert!(tmf.ch1903.easting() > 500_000.0);
    }

    #[test]
    fn test_transverse_mercator_forms_rep_map() {
        let geo = GeoParam::new("52_N_13_E").unwrap(); // Berlin
        let tmf = TransverseMercatorForms::new(&geo);

        let mut rep_map = HashMap::new();
        tmf.add_rep_map(&mut rep_map);

        // Check that all expected keys are present
        assert!(rep_map.contains_key("utmzone"));
        assert!(rep_map.contains_key("utmnorthing"));
        assert!(rep_map.contains_key("utmeasting"));
        assert!(rep_map.contains_key("utm33northing"));
        assert!(rep_map.contains_key("utm33easting"));
        assert!(rep_map.contains_key("osgb36ref"));
        assert!(rep_map.contains_key("ch1903northing"));
        assert!(rep_map.contains_key("ch1903easting"));
    }

    #[test]
    fn test_transverse_mercator_forms_outside_uk() {
        // New York - outside UK, OSGB36 should be empty
        let geo = GeoParam::new("40.7128_N_74.0060_W").unwrap();
        let tmf = TransverseMercatorForms::new(&geo);

        // OSGB36 is only valid for UK
        assert!(tmf.osgb36.grid_reference().is_empty());
    }
}
