//! Transverse Mercator coordinate transformations.
//!
//! Converts latitude/longitude geographical coordinates to various
//! Transverse Mercator projection systems including:
//! - UTM (Universal Transverse Mercator)
//! - OSGB36 (UK Ordnance Survey National Grid)
//! - CH1903 (Swiss National Grid)
//!
//! # References
//! - <http://www.posc.org/Epicentre.2_2/DataModel/ExamplesofUsage/eu_cs34h.html>
//! - <http://kanadier.gps-info.de/d-utm-gitter.htm>
//! - <http://www.gpsy.com/gpsinfo/geotoutm/>
//! - <http://www.colorado.edu/geography/gcraft/notes/gps/gps_f.html>
//! - UK Ordnance Survey grid: <http://www.gps.gov.uk/guidecontents.asp>
//! - Swiss CH1903: <http://www.swisstopo.ch/pub/down/basics/geo/system/swiss_projection_de.pdf>
//!
//! # License
//! Copyright 2005, Egil Kvaleberg <egil@kvaleberg.no>
//! Converted to Rust 2025 by Magnus Manske <magnusmanske@googlemail.com>
//!
//! This program is free software under GPL v2 or later.

/// UTM zone letters for latitude bands (from 80S to 84N).
const UTM_ZONE_LETTERS: &str = "CCCDEFGHJKLMNPQRSTUVWXXX";

/// Letters for OSGB36 grid references (excludes 'I').
const OSGB36_LETTERS: &str = "ABCDEFGHJKLMNOPQRSTUVWXYZ";

/// Ellipsoid parameters for geodetic calculations.
#[derive(Debug, Clone, Copy)]
struct Ellipsoid {
    /// Major semi-axis (equatorial radius) in meters.
    radius: f64,
    /// Square of eccentricity (e^2).
    eccentricity_sq: f64,
}

impl Ellipsoid {
    /// WGS-84 ellipsoid (default for GPS and most modern applications).
    const WGS84: Self = Self {
        radius: 6_378_137.0,
        eccentricity_sq: 0.006_694_379_990,
    };

    /// Airy 1830 ellipsoid (used for OSGB36).
    const AIRY_1830: Self = Self {
        radius: 6_377_563.396,
        eccentricity_sq: 0.006_670_54,
    };

    /// Computes the meridional arc distance from the equator to given latitude.
    fn meridional_arc(&self, lat_rad: f64) -> f64 {
        let e = self.eccentricity_sq;
        let e2 = e * e;
        let e3 = e2 * e;

        self.radius
            * ((1.0 - e / 4.0 - 3.0 * e2 / 64.0 - 5.0 * e3 / 256.0) * lat_rad
                - (3.0 * e / 8.0 + 3.0 * e2 / 32.0 + 45.0 * e3 / 1024.0) * (2.0 * lat_rad).sin()
                + (15.0 * e2 / 256.0 + 45.0 * e3 / 1024.0) * (4.0 * lat_rad).sin()
                - (35.0 * e3 / 3072.0) * (6.0 * lat_rad).sin())
    }
}

/// Projection parameters for Transverse Mercator.
#[derive(Debug, Clone, Copy)]
struct ProjectionParams {
    scale: f64,
    easting_offset: f64,
    northing_offset: f64,
    /// Additional northing offset for Southern hemisphere.
    northing_offset_south: f64,
}

impl ProjectionParams {
    /// Standard UTM projection parameters.
    const UTM: Self = Self {
        scale: 0.9996,
        easting_offset: 500_000.0,
        northing_offset: 0.0,
        northing_offset_south: 10_000_000.0,
    };

    /// OSGB36 projection parameters.
    const OSGB36: Self = Self {
        scale: 0.999_601_3,
        easting_offset: 400_000.0,
        northing_offset: -100_000.0,
        northing_offset_south: 0.0,
    };
}

/// Result of a Transverse Mercator projection.
#[derive(Debug, Clone, Default)]
pub struct ProjectionResult {
    northing: f64,
    easting: f64,
    zone: String,
}

impl ProjectionResult {
    pub const fn northing(&self) -> f64 {
        self.northing
    }

    pub const fn easting(&self) -> f64 {
        self.easting
    }

    pub fn zone(&self) -> &str {
        &self.zone
    }
}

/// Transverse Mercator projection calculator.
///
/// Converts geographic coordinates (latitude/longitude) to various
/// Transverse Mercator coordinate systems.
#[derive(Debug, Clone)]
pub struct TransverseMercator {
    result: ProjectionResult,
    ellipsoid: Ellipsoid,
    params: ProjectionParams,
}

impl Default for TransverseMercator {
    fn default() -> Self {
        Self {
            result: ProjectionResult::default(),
            ellipsoid: Ellipsoid::WGS84,
            params: ProjectionParams::UTM,
        }
    }
}

impl TransverseMercator {
    pub const fn northing(&self) -> f64 {
        self.result.northing
    }

    pub const fn easting(&self) -> f64 {
        self.result.easting
    }

    pub fn zone(&self) -> &str {
        &self.result.zone
    }

    pub fn set_zone(&mut self, zone: String) {
        self.result.zone = zone;
    }

    /// Converts latitude/longitude to UTM coordinates.
    ///
    /// Automatically determines the UTM zone based on the coordinates.
    pub fn set_utm_from_lat_lon(&mut self, latitude: f64, longitude: f64) {
        self.result.zone = Self::compute_utm_zone(latitude, longitude);
        self.lat_lon_zone_to_utm(latitude, longitude, &self.result.zone.clone());
    }

    /// Determines the UTM zone for given coordinates.
    ///
    /// Handles special cases for Norway (zone 32) and Svalbard (zones 31, 33, 35, 37).
    pub fn compute_utm_zone(latitude: f64, longitude: f64) -> String {
        let longitude = Self::normalize_longitude(longitude);

        // Handle special UTM zone exceptions
        let zone_number = match (latitude, longitude) {
            // Norway exception
            (lat, lon) if (56.0..64.0).contains(&lat) && (3.0..12.0).contains(&lon) => 32,
            // Svalbard exception
            (lat, lon) if (72.0..84.0).contains(&lat) && (0.0..42.0).contains(&lon) => {
                if lon < 9.0 {
                    31
                } else if lon < 21.0 {
                    33
                } else if lon < 33.0 {
                    35
                } else {
                    37
                }
            }
            // Standard zone calculation
            _ => ((longitude + 180.0) / 6.0) as i32 + 1,
        };

        let letter_index = ((latitude + 96.0) / 8.0) as usize;
        let zone_letter = UTM_ZONE_LETTERS
            .chars()
            .nth(letter_index.min(UTM_ZONE_LETTERS.len() - 1))
            .unwrap_or('X');

        format!("{zone_number}{zone_letter}")
    }

    /// Converts latitude/longitude to UTM coordinates in a specific zone.
    ///
    /// Returns `true` if conversion was successful, `false` if coordinates
    /// are outside the valid UTM range.
    pub fn lat_lon_zone_to_utm(&mut self, latitude: f64, longitude: f64, zone: &str) -> bool {
        let origin = Self::zone_central_meridian(zone);
        self.project_to_tm(latitude, longitude, 0.0, origin)
    }

    /// Converts latitude/longitude to OSGB36 (UK National Grid) coordinates.
    ///
    /// Returns the grid reference string (e.g., "TQ123456") or empty string
    /// if coordinates are outside the valid OSGB36 area.
    pub fn lat_lon_to_osgb36(&mut self, latitude: f64, longitude: f64) -> String {
        self.ellipsoid = Ellipsoid::AIRY_1830;
        self.params = ProjectionParams::OSGB36;

        const LATITUDE_ORIGIN: f64 = 49.0;
        const LONGITUDE_ORIGIN: f64 = -2.0;

        if !self.project_to_tm(latitude, longitude, LATITUDE_ORIGIN, LONGITUDE_ORIGIN) {
            return String::new();
        }

        self.format_osgb36_reference()
    }

    /// Formats the current northing/easting as an OSGB36 grid reference.
    fn format_osgb36_reference(&self) -> String {
        let grid_x = (self.result.easting / 100_000.0).floor() as i32;
        let grid_y = (self.result.northing / 100_000.0).floor() as i32;

        // Check if within valid OSGB36 area
        if !(0..=6).contains(&grid_x) || !(0..=12).contains(&grid_y) {
            return String::new();
        }

        let c1_index = (17 - (grid_y / 5) * 5 + grid_x / 5) as usize;
        let c2_index = (20 - (grid_y % 5) * 5 + grid_x % 5) as usize;

        let c1 = OSGB36_LETTERS.chars().nth(c1_index).unwrap_or('X');
        let c2 = OSGB36_LETTERS.chars().nth(c2_index).unwrap_or('X');

        let easting_digits = self.result.easting as i32 % 100_000;
        let northing_digits = self.result.northing as i32 % 100_000;

        format!("{c1}{c2}{easting_digits:05}{northing_digits:05}")
    }

    /// Converts latitude/longitude to CH1903 (Swiss National Grid) coordinates.
    ///
    /// Uses the approximation formula from the Swiss Federal Office of Topography.
    /// Valid range: latitude 45.5-48, longitude 5-11.
    ///
    /// Returns `true` if conversion was successful, `false` if coordinates
    /// are outside the valid range.
    pub fn lat_lon_to_ch1903(&mut self, latitude: f64, longitude: f64) -> bool {
        if !(45.5..=48.0).contains(&latitude) || !(5.0..=11.0).contains(&longitude) {
            self.result.easting = 0.0;
            self.result.northing = 0.0;
            return false;
        }

        // Convert to arc-seconds and normalize to Bern origin
        let lat_aux = (latitude * 3600.0 - 169_028.66) / 10_000.0;
        let lon_aux = (longitude * 3600.0 - 26_782.5) / 10_000.0;

        // Polynomial approximation (Swiss Federal Office of Topography formula)
        self.result.northing = 200_147.07
            + 308_807.95 * lat_aux
            + 3_745.25 * lon_aux.powi(2)
            + 76.63 * lat_aux.powi(2)
            - 194.56 * lon_aux.powi(2) * lat_aux
            + 119.79 * lat_aux.powi(3);

        self.result.easting = 600_072.37 + 211_455.93 * lon_aux
            - 10_938.51 * lon_aux * lat_aux
            - 0.36 * lon_aux * lat_aux.powi(2)
            - 44.54 * lon_aux.powi(3);

        true
    }

    /// Computes the central meridian for a UTM zone.
    fn zone_central_meridian(zone: &str) -> f64 {
        let zone_num: i32 = zone
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect::<String>()
            .parse()
            .unwrap_or(1);

        (zone_num - 1) as f64 * 6.0 - 180.0 + 3.0
    }

    /// Core Transverse Mercator projection calculation.
    ///
    /// Projects geographic coordinates to Transverse Mercator using the
    /// current ellipsoid and projection parameters.
    fn project_to_tm(
        &mut self,
        latitude: f64,
        longitude: f64,
        latitude_origin: f64,
        longitude_origin: f64,
    ) -> bool {
        // Validate input ranges
        if !(-180.0..=180.0).contains(&longitude) || !(-80.0..=84.0).contains(&latitude) {
            return false;
        }

        let longitude = Self::normalize_longitude(longitude);
        let lat_rad = latitude.to_radians();

        let e = self.ellipsoid.eccentricity_sq;
        let e_prime_sq = e / (1.0 - e);

        let sin_lat = lat_rad.sin();
        let cos_lat = lat_rad.cos();
        let tan_lat = lat_rad.tan();

        // Radius of curvature in prime vertical
        let n = self.ellipsoid.radius / (1.0 - e * sin_lat.powi(2)).sqrt();

        let t = tan_lat.powi(2);
        let c = e_prime_sq * cos_lat.powi(2);
        let a = (longitude - longitude_origin).to_radians() * cos_lat;

        let m = self.ellipsoid.meridional_arc(lat_rad);
        let m0 = if latitude_origin != 0.0 {
            self.ellipsoid.meridional_arc(latitude_origin.to_radians())
        } else {
            0.0
        };

        // Compute northing
        let northing = self.params.northing_offset
            + self.params.scale
                * ((m - m0)
                    + n * tan_lat
                        * (a.powi(2) / 2.0
                            + (5.0 - t + 9.0 * c + 4.0 * c.powi(2)) * a.powi(4) / 24.0
                            + (61.0 - 58.0 * t + t.powi(2) + 600.0 * c - 330.0 * e_prime_sq)
                                * a.powi(6)
                                / 720.0));

        // Compute easting
        let easting = self.params.easting_offset
            + self.params.scale
                * n
                * (a + (1.0 - t + c) * a.powi(3) / 6.0
                    + (5.0 - 18.0 * t + t.powi(2) + 72.0 * c - 58.0 * e_prime_sq) * a.powi(5)
                        / 120.0);

        // Apply southern hemisphere offset if needed
        self.result.northing = if latitude < 0.0 {
            northing + self.params.northing_offset_south
        } else {
            northing
        };
        self.result.easting = easting;

        true
    }

    /// Normalizes longitude to the range [-180, 180).
    fn normalize_longitude(longitude: f64) -> f64 {
        longitude - ((longitude + 180.0) / 360.0).floor() * 360.0
    }
}

/// OSGB36 (UK Ordnance Survey National Grid) coordinate converter.
///
/// Wraps `TransverseMercator` with OSGB36-specific parameters.
#[derive(Debug, Clone, Default)]
pub struct OSGB36 {
    tm: TransverseMercator,
}

impl OSGB36 {
    pub const fn northing(&self) -> f64 {
        self.tm.result.northing
    }

    pub const fn easting(&self) -> f64 {
        self.tm.result.easting
    }

    /// Converts latitude/longitude to OSGB36 coordinates.
    ///
    /// Returns the grid reference string or empty string if outside valid area.
    pub fn lat_lon_to_osgb36(&mut self, latitude: f64, longitude: f64) -> String {
        self.tm.lat_lon_to_osgb36(latitude, longitude)
    }
}

/// CH1903 (Swiss National Grid) coordinate converter.
///
/// Wraps `TransverseMercator` with CH1903-specific parameters.
#[derive(Debug, Clone, Default)]
pub struct CH1903 {
    tm: TransverseMercator,
}

impl CH1903 {
    pub const fn northing(&self) -> f64 {
        self.tm.result.northing
    }

    pub const fn easting(&self) -> f64 {
        self.tm.result.easting
    }

    /// Converts latitude/longitude to CH1903 coordinates.
    ///
    /// Returns `true` if coordinates are within valid Swiss range.
    pub fn lat_lon_to_ch1903(&mut self, latitude: f64, longitude: f64) -> bool {
        self.tm.lat_lon_to_ch1903(latitude, longitude)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f64::consts::PI;

    #[test]
    fn test_utm_zone_calculation() {
        // Standard zone calculation - New York area
        let zone1 = TransverseMercator::compute_utm_zone(40.0, -74.0);
        assert!(zone1.starts_with("18"));

        // Equator at prime meridian
        let zone2 = TransverseMercator::compute_utm_zone(0.0, 0.0);
        assert!(zone2.starts_with("31"));
    }

    #[test]
    fn test_utm_conversion() {
        let mut tm = TransverseMercator::default();
        tm.set_utm_from_lat_lon(40.7128, -74.0060);

        assert!(tm.northing() > 0.0);
        assert!(tm.easting() > 0.0);
        assert!(!tm.zone().is_empty());
    }

    #[test]
    fn test_osgb36_conversion() {
        let mut tm = TransverseMercator::default();

        // London coordinates
        let result = tm.lat_lon_to_osgb36(51.5074, -0.1278);
        assert!(!result.is_empty());
    }

    #[test]
    fn test_ch1903_conversion() {
        let mut tm = TransverseMercator::default();

        // Bern, Switzerland
        assert!(tm.lat_lon_to_ch1903(46.9480, 7.4474));
        assert!(tm.northing() > 0.0);
        assert!(tm.easting() > 0.0);

        // Outside valid range
        assert!(!tm.lat_lon_to_ch1903(50.0, 0.0));
    }

    #[test]
    fn test_deg_rad_conversion() {
        let deg = 45.0;
        let rad = f64::to_radians(deg);
        let back = rad * 180.0 / PI;

        assert!((deg - back).abs() < 1e-10);
    }

    #[test]
    fn test_normalize_longitude() {
        assert!((TransverseMercator::normalize_longitude(0.0) - 0.0).abs() < 1e-10);
        assert!((TransverseMercator::normalize_longitude(180.0) - (-180.0)).abs() < 1e-10);
        assert!((TransverseMercator::normalize_longitude(-180.0) - (-180.0)).abs() < 1e-10);
        assert!((TransverseMercator::normalize_longitude(360.0) - 0.0).abs() < 1e-10);
        assert!((TransverseMercator::normalize_longitude(540.0) - (-180.0)).abs() < 1e-10);
    }

    #[test]
    fn test_ellipsoid_meridional_arc() {
        let wgs84 = Ellipsoid::WGS84;

        // At equator, meridional arc should be 0
        assert!((wgs84.meridional_arc(0.0) - 0.0).abs() < 1e-10);

        // At 45 degrees, should be approximately half the distance to pole
        let arc_45 = wgs84.meridional_arc(45.0_f64.to_radians());
        assert!(arc_45 > 4_900_000.0 && arc_45 < 5_100_000.0);
    }
}
