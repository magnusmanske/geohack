use crate::geo_param::GeoParam;
use crate::traverse_mercator::{CH1903, OSGB36, TransverseMercator};
use std::collections::HashMap;

#[derive(Debug, Clone, Default)]
pub struct TransverseMercatorForms {
    utm: TransverseMercator,
    utm33: TransverseMercator,
    osgb36: OSGB36,
    osgb36ref: String,
    ch1903: CH1903,
}

impl TransverseMercatorForms {
    pub fn new(p: &GeoParam) -> Self {
        /*
         *  Convert coordinates to various Transverse Mercator forms
         */

        /* standard UTM */
        let mut utm = TransverseMercator::default();
        utm.lat_lon_to_utm(p.latdeg(), p.londeg());
        utm.zone = utm.lat_lon_to_utm_zone(p.latdeg(), p.londeg());

        /* fixed UTM as used by iNatur */
        let mut utm33 = TransverseMercator::default();
        utm33.lat_lon_zone_to_utm(p.latdeg(), p.londeg(), "33V");

        /*  UK National Grid, see http://www.gps.gov.uk/guide7.asp
         *  central meridian 47N 2W, offset 100km N 400km W */
        let mut osgb36 = OSGB36::default();
        let osgb36ref = osgb36.lat_lon_to_osgb36(p.latdeg(), p.londeg());

        /* Swiss traditional national grid */
        let mut ch1903 = CH1903::default();
        ch1903.lat_lon_to_ch1903(p.latdeg(), p.londeg());
        Self {
            utm,
            utm33,
            osgb36,
            osgb36ref,
            ch1903,
        }
    }

    pub fn add_rep_map(&self, rep_map: &mut HashMap<String, String>) {
        rep_map.insert("utmzone".to_string(), self.utm.zone.clone());
        rep_map.insert(
            "utmnorthing".to_string(),
            self.utm.northing.round().to_string(),
        );
        rep_map.insert(
            "utmeasting".to_string(),
            self.utm.easting.round().to_string(),
        );
        rep_map.insert(
            "utm33northing".to_string(),
            self.utm33.northing.round().to_string(),
        );
        rep_map.insert(
            "utm33easting".to_string(),
            self.utm33.easting.round().to_string(),
        );
        rep_map.insert("osgb36ref".to_string(), self.osgb36ref.to_string());
        rep_map.insert(
            "osgb36northing".to_string(),
            self.osgb36.northing.round().to_string(),
        );
        rep_map.insert(
            "osgb36easting".to_string(),
            self.osgb36.easting.round().to_string(),
        );
        rep_map.insert(
            "ch1903northing".to_string(),
            self.ch1903.northing.round().to_string(),
        );
        rep_map.insert(
            "ch1903easting".to_string(),
            self.ch1903.easting.round().to_string(),
        );
    }
}
