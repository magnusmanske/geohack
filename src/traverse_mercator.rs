/*!
 *  Projections for the map-source placeholders:
 *
 *  - UTM (WGS-84): delegated to the `utm` crate
 *  - OSGB36 (UK national grid): delegated to the `lonlat_bng` crate, which
 *    applies the official OSTN15 datum transformation (WGS84 -> OSGB36).
 *    The previous hand-rolled code (inherited from the PHP original) skipped
 *    the datum shift entirely, making its output ~100 m off across the UK.
 *  - CH1903 (Swiss national grid): official swisstopo approximation formula,
 *    for which no suitable crate exists
 *
 *  The file name typo (traverse vs transverse) is historical.
 *
 *  ----------------------------------------------------------------------
 *
 *  Grid-reference lettering and CH1903 formula derived from code
 *  Copyright 2005, Egil Kvaleberg <egil@kvaleberg.no>
 *  Converted to Rust 2025 by <Magnus Manske> <magnusmanske@googlemail.com>
 *
 *  This program is free software; you can redistribute it and/or modify
 *  it under the terms of the GNU General Public License as published by
 *  the Free Software Foundation; either version 2 of the License, or
 *  (at your option) any later version.
 */

/// UTM (WGS-84) coordinates
#[derive(Debug, Clone, Default)]
pub struct Utm {
    northing: f64,
    easting: f64,
    zone: String,
}

impl Utm {
    /// Convert latitude/longitude to UTM, auto-detecting the zone
    /// (including the Norway/Svalbard exceptions)
    pub fn from_lat_lon(latitude: f64, longitude: f64) -> Self {
        let longitude = normalize_longitude(longitude);
        let zone_number = utm::lat_lon_to_zone_number(latitude, longitude);
        // Outside the UTM latitude band, keep the letters the original code
        // produced ('X' far north, 'C' far south)
        let zone_letter =
            utm::lat_to_zone_letter(latitude).unwrap_or(if latitude > 0.0 { 'X' } else { 'C' });
        let (northing, easting) = northing_easting(latitude, longitude, zone_number);
        Self {
            northing,
            easting,
            zone: format!("{zone_number}{zone_letter}"),
        }
    }

    /// Convert latitude/longitude to UTM in a fixed zone (iNatur uses zone 33)
    pub fn from_lat_lon_forced_zone(latitude: f64, longitude: f64, zone_number: u8) -> Self {
        let longitude = normalize_longitude(longitude);
        let (northing, easting) = northing_easting(latitude, longitude, zone_number);
        Self {
            northing,
            easting,
            zone: zone_number.to_string(),
        }
    }

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

/// UTM is not defined outside latitude -80..84; return zeros there,
/// like the original code did
fn northing_easting(latitude: f64, longitude: f64, zone_number: u8) -> (f64, f64) {
    if !(-80.0..=84.0).contains(&latitude) {
        return (0.0, 0.0);
    }
    let (northing, easting, _meridian_convergence) =
        utm::to_utm_wgs84(latitude, longitude, zone_number);
    // The `utm` crate applies the 10,000,000 m southern false northing at
    // exactly 0.0 latitude (its check is `latitude > 0.0`); treat the equator
    // as northern hemisphere instead. At the equator the raw northing is 0,
    // so subtracting the false northing is exact.
    if latitude == 0.0 {
        (northing - 10_000_000.0, easting)
    } else {
        (northing, easting)
    }
}

/// Normalize longitude to -180..180 (GeoParam allows up to +/-360)
fn normalize_longitude(longitude: f64) -> f64 {
    longitude - ((longitude + 180.0) / 360.0).floor() * 360.0
}

/// OSGB36 (UK national grid) coordinates and grid reference
#[derive(Debug, Clone, Default)]
pub struct OSGB36 {
    northing: f64,
    easting: f64,
    grid_reference: String,
}

impl OSGB36 {
    /// Convert WGS84 latitude/longitude to OSGB36. Outside the OSTN15
    /// coverage area (essentially, outside Great Britain) all values are
    /// zero/empty.
    pub fn from_lat_lon(latitude: f64, longitude: f64) -> Self {
        match lonlat_bng::convert_osgb36(longitude, latitude) {
            Ok((easting, northing)) => Self {
                northing,
                easting,
                grid_reference: Self::make_grid_reference(easting, northing),
            },
            Err(_) => Self::default(),
        }
    }

    /// Two grid letters plus 5+5 digits (1 m resolution), e.g. "TQ3000480446"
    fn make_grid_reference(easting: f64, northing: f64) -> String {
        /* fix by Roger W Haworth */
        let grid_x = (easting / 100_000.0).floor() as i32;
        let grid_y = (northing / 100_000.0).floor() as i32;
        if !(0..=6).contains(&grid_x) || !(0..=12).contains(&grid_y) {
            /* outside area for OSGB36 */
            return String::new();
        }

        /*             0000000000111111111122222 */
        /*             0123456789012345678901234 */
        const LETTERS: &[u8] = b"ABCDEFGHJKLMNOPQRSTUVWXYZ";
        let c1 = LETTERS[((17 - (grid_y / 5) * 5) + (grid_x / 5)) as usize] as char;
        let c2 = LETTERS[((20 - (grid_y % 5) * 5) + (grid_x % 5)) as usize] as char;

        format!(
            "{}{}{:05}{:05}",
            c1,
            c2,
            easting as i32 % 100_000,
            northing as i32 % 100_000
        )
    }

    pub const fn northing(&self) -> f64 {
        self.northing
    }

    pub const fn easting(&self) -> f64 {
        self.easting
    }

    pub fn grid_reference(&self) -> &str {
        &self.grid_reference
    }
}

/// CH1903 (Swiss traditional national grid) coordinates
#[derive(Debug, Clone, Copy, Default)]
pub struct CH1903 {
    northing: f64,
    easting: f64,
}

impl CH1903 {
    /// Convert latitude/longitude to CH1903.
    /// Assumed range is latitude 45.5..48 and longitude 5..11; zero outside.
    /// Code by [[de:Benutzer:Meleager]]
    pub fn from_lat_lon(latitude: f64, longitude: f64) -> Self {
        if !(45.5..=48.0).contains(&latitude) || !(5.0..=11.0).contains(&longitude) {
            /* outside reasonable range */
            return Self::default();
        }

        /* Approximation formula according to */
        /* http://www.swisstopo.ch/pub/down/basics/geo/system/swiss_projection_de.pdf */
        /* chapter 4.1, page 11. */

        let ps = latitude * 3600.0;
        let ls = longitude * 3600.0;

        let pp = (ps - 169028.66) / 10000.0;
        let lp = (ls - 26782.5) / 10000.0;

        let northing = 200147.07 + 308807.95 * pp + 3745.25 * lp * lp + 76.63 * pp * pp
            - 194.56 * lp * lp * pp
            + 119.79 * pp * pp * pp;

        let easting = 600072.37 + 211455.93 * lp
            - 10938.51 * lp * pp
            - 0.36 * lp * pp * pp
            - 44.54 * lp * lp * lp;

        Self { northing, easting }
    }

    pub const fn northing(&self) -> f64 {
        self.northing
    }

    pub const fn easting(&self) -> f64 {
        self.easting
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_utm_zone_calculation() {
        assert_eq!(Utm::from_lat_lon(40.0, -74.0).zone(), "18T");
        assert_eq!(Utm::from_lat_lon(0.0, 0.0).zone(), "31N");
        // Norway/Svalbard exceptions
        assert_eq!(Utm::from_lat_lon(59.9139, 10.7522).zone(), "32V");
        assert_eq!(Utm::from_lat_lon(78.2232, 15.6267).zone(), "33X");
    }

    #[test]
    fn test_utm_conversion() {
        let utm = Utm::from_lat_lon(40.7128, -74.0060);
        assert!(utm.northing() > 0.0);
        assert!(utm.easting() > 0.0);
        assert!(!utm.zone().is_empty());
    }

    #[test]
    fn test_utm_equator_northing_is_zero() {
        // Equator must use the northern-hemisphere convention (northing 0),
        // not the 10,000,000 m southern false northing
        let utm = Utm::from_lat_lon(0.0, 0.0);
        assert!(utm.northing().abs() < 1.0);
    }

    #[test]
    fn test_utm_out_of_range_latitude() {
        let utm = Utm::from_lat_lon(85.0, 10.0);
        assert_eq!(utm.northing(), 0.0);
        assert_eq!(utm.easting(), 0.0);
        assert_eq!(utm.zone(), "32X");
    }

    #[test]
    fn test_utm_longitude_normalization() {
        // +200 degrees == -160 degrees
        let utm_a = Utm::from_lat_lon(10.0, 200.0);
        let utm_b = Utm::from_lat_lon(10.0, -160.0);
        assert_eq!(utm_a.zone(), utm_b.zone());
        assert_eq!(utm_a.easting(), utm_b.easting());
    }

    #[test]
    fn test_osgb36_conversion() {
        // Trafalgar Square; reference value verified against OS conversion
        let osgb36 = OSGB36::from_lat_lon(51.5080, -0.1281);
        assert_eq!(osgb36.grid_reference(), "TQ3000480446");
        assert!((osgb36.easting() - 530004.0).abs() < 1.0);
        assert!((osgb36.northing() - 180446.0).abs() < 1.0);
    }

    #[test]
    fn test_osgb36_outside_uk() {
        let osgb36 = OSGB36::from_lat_lon(40.7128, -74.0060);
        assert_eq!(osgb36.grid_reference(), "");
        assert_eq!(osgb36.northing(), 0.0);
        assert_eq!(osgb36.easting(), 0.0);
    }

    #[test]
    fn test_ch1903_conversion() {
        // Bern
        let ch1903 = CH1903::from_lat_lon(46.9480, 7.4474);
        assert!(ch1903.northing() > 100_000.0);
        assert!(ch1903.easting() > 500_000.0);

        // Out of range
        let ch1903_far = CH1903::from_lat_lon(50.0, 0.0);
        assert_eq!(ch1903_far.northing(), 0.0);
        assert_eq!(ch1903_far.easting(), 0.0);
    }
}
