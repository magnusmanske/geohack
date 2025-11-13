// TODO use https://docs.rs/lonlat_bng/latest/lonlat_bng/ where possible

/**
 *  Convert latitude longitude geographical coordinates to
 *  Transverse Mercator coordinates.
 *
 *  Uses the WGS-84 ellipsoid by default
 *
 *  See also:
 *  http://www.posc.org/Epicentre.2_2/DataModel/ExamplesofUsage/eu_cs34h.html
 *  http://kanadier.gps-info.de/d-utm-gitter.htm
 *  http://www.gpsy.com/gpsinfo/geotoutm/
 *  http://www.colorado.edu/geography/gcraft/notes/gps/gps_f.html
 *  http://search.cpan.org/src/GRAHAMC/Geo-Coordinates-UTM-0.05/
 *  UK Ordnance Survey grid (OSBG36): http://www.gps.gov.uk/guidecontents.asp
 *  Swiss CH1903: http://www.gps.gov.uk/guidecontents.asp
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
use std::f64::consts::PI;

/**
 *  Transverse Mercator transformations
 */
#[derive(Debug, Clone)]
pub struct TransverseMercator {
    /* public: */
    pub northing: f64,
    pub easting: f64,
    pub zone: String,

    /* Reference Ellipsoid, default to WGS-84 */
    pub radius: f64,       /* major semi axis = a */
    pub eccentricity: f64, /* square of eccentricity */

    /* Flattening = f = (a-b) / a */
    /* Inverse flattening = 1/f = 298.2572236 */
    /* Minor semi axis b = a*(1-f) */
    /* Eccentricity e = sqrt(a^2 - b^2)/a = 0.081819190843 */

    /* Transverse Mercator parameters */
    pub scale: f64,
    pub easting_offset: f64,
    pub northing_offset: f64,
    pub northing_offset_south: f64, /* for Southern hemisphere */
}

impl Default for TransverseMercator {
    fn default() -> Self {
        TransverseMercator {
            northing: 0.0,
            easting: 0.0,
            zone: String::new(),
            radius: 6378137.0,            /* major semi axis = a */
            eccentricity: 0.006694379990, /* square of eccentricity */
            scale: 0.9996,
            easting_offset: 500000.0,
            northing_offset: 0.0,
            northing_offset_south: 10000000.0,
        }
    }
}

impl TransverseMercator {
    /**
     *  Convert latitude, longitude in decimal degrees to
     *  UTM Zone, Easting, and Northing
     */
    pub fn lat_lon_to_utm(&mut self, latitude: f64, longitude: f64) {
        self.zone = self.lat_lon_to_utm_zone(latitude, longitude);
        self.lat_lon_zone_to_utm(latitude, longitude, &self.zone.clone());
    }

    /**
     *  Find UTM zone from latitude and longitude
     */
    pub fn lat_lon_to_utm_zone(&self, latitude: f64, longitude: f64) -> String {
        let longitude2 = longitude - ((longitude + 180.0) / 360.0).floor() * 360.0;

        let zone = if (56.0..64.0).contains(&latitude) && (3.0..12.0).contains(&longitude2) {
            32
        } else if (72.0..84.0).contains(&latitude) && (0.0..42.0).contains(&longitude2) {
            if longitude2 < 9.0 {
                31
            } else if longitude2 < 21.0 {
                33
            } else if longitude2 < 33.0 {
                35
            } else {
                37
            }
        } else {
            ((longitude2 + 180.0) / 6.0) as i32 + 1
        };

        let c = ((latitude + 96.0) / 8.0) as usize;
        /* 000000000011111111112222 */
        /* 012345678901234567890134 */
        let letters = "CCCDEFGHJKLMNPQRSTUVWXXX";
        let letter = letters.chars().nth(c).unwrap_or('X');

        format!("{}{}", zone, letter)
    }

    /**
     *  Convert latitude, longitude in decimal degrees to
     *  UTM Easting and Northing in a specific zone
     *
     *  \return false if problems
     */
    pub fn lat_lon_zone_to_utm(&mut self, latitude: f64, longitude: f64, zone: &str) -> bool {
        self.lat_lon_origin_to_tm(latitude, longitude, 0.0, Self::utmzone_origin(zone))
    }

    /**
     *  Convert latitude, longitude in decimal degrees to
     *  OSBG36 Easting and Northing
     */
    pub fn lat_lon_to_osgb36(&mut self, latitude: f64, longitude: f64) -> String {
        /* Airy 1830 ellipsoid */
        self.radius = 6377563.396;
        /* inverse flattening 1/f: 299.3249647 */
        self.eccentricity = 0.00667054; /* square of eccentricity */

        self.scale = 0.9996013;
        self.easting_offset = 400000.0;
        self.northing_offset = -100000.0;
        self.northing_offset_south = 0.0;

        let latitude_origin = 49.0;
        let longitude_origin = -2.0;

        if !self.lat_lon_origin_to_tm(latitude, longitude, latitude_origin, longitude_origin) {
            return String::new();
        }

        /* fix by Roger W Haworth */
        let grid_x = (self.easting / 100000.0).floor() as i32;
        let grid_y = (self.northing / 100000.0).floor() as i32;

        if !(0..=6).contains(&grid_x) || !(0..=12).contains(&grid_y) {
            /* outside area for OSGB36 */
            return String::new();
        }

        /*             0000000000111111111122222 */
        /*             0123456789012345678901234 */
        let letters = "ABCDEFGHJKLMNOPQRSTUVWXYZ";

        let c1_index = (17 - (grid_y / 5) * 5) + (grid_x / 5);
        let c2_index = (20 - (grid_y % 5) * 5) + (grid_x % 5);

        let c1 = letters.chars().nth(c1_index as usize).unwrap_or('X');
        let c2 = letters.chars().nth(c2_index as usize).unwrap_or('X');

        let e = format!("{:05}", self.easting as i32 % 100000);
        let n = format!("{:05}", self.northing as i32 % 100000);

        format!("{}{}{}{}", c1, c2, e, n)
    }

    /**
     *  Convert latitude, longitude in decimal degrees to
     *  CH1903 Easting and Northing
     *  Assumed range is latitude 45.5 .. 48 and logitude 5 - 11
     *  Code by [[de:Benutzer:Meleager]]
     */
    pub fn lat_lon_to_ch1903(&mut self, latitude: f64, longitude: f64) -> bool {
        if !(45.5..=48.0).contains(&latitude) || !(5.0..=11.0).contains(&longitude) {
            /* outside reasonable range */
            self.easting = 0.0;
            self.northing = 0.0;
            return false;
        }

        /* Approximation formula according to  */
        /* http://www.swisstopo.ch/pub/down/basics/geo/system/swiss_projection_de.pdf */
        /* chapter 4.1, page 11. */

        let ps = latitude * 3600.0;
        let ls = longitude * 3600.0;

        let pp = (ps - 169028.66) / 10000.0;
        let lp = (ls - 26782.5) / 10000.0;

        self.northing = 200147.07 + 308807.95 * pp + 3745.25 * lp * lp + 76.63 * pp * pp
            - 194.56 * lp * lp * pp
            + 119.79 * pp * pp * pp;

        self.easting = 600072.37 + 211455.93 * lp
            - 10938.51 * lp * pp
            - 0.36 * lp * pp * pp
            - 44.54 * lp * lp * lp;

        true
    }

    /*	Kvalberg code
        function LatLon2CH1903( $latitude, $longitude )
        {
            if ($latitude < 45.5 or $latitude > 48
             or $longitude < 5.0 or $longitude > 11) {
                // outside reasonable range
                $this->Easting = "";
                $this->Northing = "";
                return false;
            }

            // ellipsoid: Bessel 1841
            $this->Radius = 6377397.155;
            // 299.1528128
            $this->Eccentricity = 0.006674372;

            $this->Scale = 1.0;
            $this->Easting_Offset =   600000.0;
            $this->Northing_Offset =  200000.0;
            $this->Northing_Offset_South = 0.0;

            $latitude_origin  = 46.95240555556;
            $longitude_origin =  7.43958333333;

            $this->LatLonOrigin2TM( $latitude, $longitude,
                    $latitude_origin, $longitude_origin );
        }
    */

    fn utmzone_origin(zone: &str) -> f64 {
        let zone_num = zone
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse::<i32>()
            .unwrap_or(1);

        (zone_num - 1) as f64 * 6.0 - 180.0 + 3.0
    }

    fn find_m(&self, lat_rad: f64) -> f64 {
        let e = self.eccentricity;

        self.radius
            * ((1.0 - e / 4.0 - 3.0 * e * e / 64.0 - 5.0 * e * e * e / 256.0) * lat_rad
                - (3.0 * e / 8.0 + 3.0 * e * e / 32.0 + 45.0 * e * e * e / 1024.0)
                    * (2.0 * lat_rad).sin()
                + (15.0 * e * e / 256.0 + 45.0 * e * e * e / 1024.0) * (4.0 * lat_rad).sin()
                - (35.0 * e * e * e / 3072.0) * (6.0 * lat_rad).sin())
    }

    fn deg2rad(deg: f64) -> f64 {
        (PI / 180.0) * deg
    }

    /**
     *  Convert latitude, longitude in decimal degrees to
     *  TM Easting and Northing based on a specified origin
     *
     *  \return false if problems
     */
    pub fn lat_lon_origin_to_tm(
        &mut self,
        latitude: f64,
        longitude: f64,
        latitude_origin: f64,
        longitude_origin: f64,
    ) -> bool {
        if !(-180.0..=180.0).contains(&longitude) || !(-80.0..=84.0).contains(&latitude) {
            // UTM not defined in this range
            return false;
        }

        let longitude2 = longitude - ((longitude + 180.0) / 360.0).floor() * 360.0;

        let lat_rad = Self::deg2rad(latitude);

        let e = self.eccentricity;
        let e_prime_sq = e / (1.0 - e);

        let v = self.radius / (1.0 - e * lat_rad.sin() * lat_rad.sin()).sqrt();
        let t_val = lat_rad.tan().powi(2);
        let c = e_prime_sq * lat_rad.cos().powi(2);
        let a = Self::deg2rad(longitude2 - longitude_origin) * lat_rad.cos();
        let m = self.find_m(lat_rad);

        let m0 = if latitude_origin != 0.0 {
            self.find_m(Self::deg2rad(latitude_origin))
        } else {
            0.0
        };

        let northing = self.northing_offset
            + self.scale
                * ((m - m0)
                    + v * lat_rad.tan()
                        * (a * a / 2.0
                            + (5.0 - t_val + 9.0 * c + 4.0 * c * c) * a.powi(4) / 24.0
                            + (61.0 - 58.0 * t_val + t_val * t_val + 600.0 * c
                                - 330.0 * e_prime_sq)
                                * a.powi(6)
                                / 720.0));

        let easting = self.easting_offset
            + self.scale
                * v
                * (a + (1.0 - t_val + c) * a.powi(3) / 6.0
                    + (5.0 - 18.0 * t_val + t_val.powi(2) + 72.0 * c - 58.0 * e_prime_sq)
                        * a.powi(5)
                        / 120.0);

        // FIXME: Uze zone_letter
        // if (ord($zone_letter) < ord('N'))
        let northing = if latitude < 0.0 {
            northing + self.northing_offset_south
        } else {
            northing
        };

        self.northing = northing;
        self.easting = easting;

        true
    }
}

// Additional structures for specialized coordinate systems
#[derive(Debug, Clone)]
pub struct OSGB36 {
    pub tm: TransverseMercator,
    pub northing: f64,
    pub easting: f64,
}

impl Default for OSGB36 {
    fn default() -> Self {
        let tm = TransverseMercator {
            radius: 6377563.396,      // Airy 1830 ellipsoid
            eccentricity: 0.00667054, // square of eccentricity
            scale: 0.9996013,         // inverse flattening 1/f: 299.3249647
            easting_offset: 400000.0,
            northing_offset: -100000.0,
            northing_offset_south: 0.0,
            ..Default::default()
        };
        OSGB36 {
            tm,
            northing: 0.0,
            easting: 0.0,
        }
    }
}

impl OSGB36 {
    pub fn lat_lon_to_osgb36(&mut self, latitude: f64, longitude: f64) -> String {
        let result = self.tm.lat_lon_to_osgb36(latitude, longitude);
        self.northing = self.tm.northing;
        self.easting = self.tm.easting;
        result
    }
}

#[derive(Debug, Clone)]
pub struct CH1903 {
    pub tm: TransverseMercator,
    pub northing: f64,
    pub easting: f64,
}

impl Default for CH1903 {
    fn default() -> Self {
        CH1903 {
            tm: TransverseMercator::default(),
            northing: 0.0,
            easting: 0.0,
        }
    }
}

impl CH1903 {
    pub fn lat_lon_to_ch1903(&mut self, latitude: f64, longitude: f64) -> bool {
        let result = self.tm.lat_lon_to_ch1903(latitude, longitude);
        self.northing = self.tm.northing;
        self.easting = self.tm.easting;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utm_zone_calculation() {
        let tm = TransverseMercator::default();

        // Test standard zone calculation
        let zone1 = tm.lat_lon_to_utm_zone(40.0, -74.0);
        assert!(zone1.starts_with("18"));

        // Test equator
        let zone2 = tm.lat_lon_to_utm_zone(0.0, 0.0);
        assert!(zone2.starts_with("31"));
    }

    #[test]
    fn test_utm_conversion() {
        let mut tm = TransverseMercator::default();
        tm.lat_lon_to_utm(40.7128, -74.0060);

        // Check that values are set
        assert!(tm.northing > 0.0);
        assert!(tm.easting > 0.0);
        assert!(!tm.zone.is_empty());
    }

    #[test]
    fn test_osgb36_conversion() {
        let mut tm = TransverseMercator::default();

        // Test London coordinates
        let result = tm.lat_lon_to_osgb36(51.5074, -0.1278);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_ch1903_conversion() {
        let mut tm = TransverseMercator::default();

        // Test Swiss coordinates (Bern)
        let result1 = tm.lat_lon_to_ch1903(46.9480, 7.4474);
        assert!(result1);
        assert!(tm.northing > 0.0);
        assert!(tm.easting > 0.0);

        // Test out of range
        let result2 = tm.lat_lon_to_ch1903(50.0, 0.0);
        assert!(!result2);
    }

    #[test]
    fn test_deg_rad_conversion() {
        fn rad2deg(rad: f64) -> f64 {
            rad * (180.0 / PI)
        }

        let deg = 45.0;
        let rad = TransverseMercator::deg2rad(deg);
        let back = rad2deg(rad);

        assert!((deg - back).abs() < 1e-10);
    }
}
