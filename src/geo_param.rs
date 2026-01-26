use anyhow::{Result, anyhow};
use std::collections::HashMap;

/// Parse geographic parameters
#[derive(Debug, Clone, Default)]
pub struct GeoParam {
    latdeg: f64,
    londeg: f64,
    latdeg_min: f64,
    londeg_min: f64,
    latdeg_max: f64,
    londeg_max: f64,
    pieces: Vec<String>,
    coor: Vec<String>,
}

impl GeoParam {
    /// Constructor: Read coordinates, and if there is a range, read the range
    pub fn new(param: &str) -> Result<Self> {
        // Replace underscores with spaces and split
        let pieces: Vec<String> = param
            .replace('_', " ")
            .split_whitespace()
            .map(|s| {
                // Only replace 'O' with 'E' for standalone direction indicators
                // (German "Ost" for East), not in attribute values like region:JO
                if s == "O" || s == "o" {
                    "E".to_string()
                } else {
                    s.to_string()
                }
            })
            .collect();

        let mut geo = Self {
            pieces,
            ..Default::default()
        };

        geo.get_coor()?;
        geo.init_min_max();

        // Handle coordinate ranges (e.g., "40 N 74 W to 41 N 73 W")
        if geo.pieces.first().map(|s| s.as_str()) == Some("to") {
            geo.pieces.remove(0);
            geo.get_coor()?;
            geo.update_range_bounds();
        }

        Ok(geo)
    }

    /// Initialize min/max bounds from current coordinates
    const fn init_min_max(&mut self) {
        self.latdeg_min = self.latdeg;
        self.latdeg_max = self.latdeg;
        self.londeg_min = self.londeg;
        self.londeg_max = self.londeg;
    }

    /// Update range bounds after parsing second coordinate in a range
    fn update_range_bounds(&mut self) {
        if self.latdeg < self.latdeg_max {
            self.latdeg_min = self.latdeg;
        } else {
            self.latdeg_max = self.latdeg;
        }

        if self.londeg < self.londeg_max {
            self.londeg_min = self.londeg;
        } else {
            self.londeg_max = self.londeg;
        }

        // Set coordinates to center of range
        self.latdeg = (self.latdeg_max + self.latdeg_min) / 2.0;
        self.londeg = (self.londeg_max + self.londeg_min) / 2.0;
        self.coor.clear();
    }

    pub const fn latdeg(&self) -> f64 {
        self.latdeg
    }

    pub const fn londeg(&self) -> f64 {
        self.londeg
    }

    /// Parse a piece as f64, defaulting to 0.0
    fn parse_piece(&mut self) -> f64 {
        self.pieces.remove(0).parse().unwrap_or(0.0)
    }

    /// Get a set of coordinates from parameters
    fn get_coor(&mut self) -> Result<()> {
        if self.pieces.is_empty() {
            return Err(anyhow!("No coordinates provided"));
        }

        let (lat_ns, lon_ew, latmin, lonmin, latsec, lonsec) = self.parse_coordinate_format()?;

        self.validate_ranges(latmin, lonmin, latsec, lonsec)?;
        self.convert_to_decimal_degrees(&lat_ns, &lon_ew, latmin, lonmin, latsec, lonsec);
        Ok(())
    }

    /// Parse coordinates based on format and return direction indicators and minute/second values
    fn parse_coordinate_format(&mut self) -> Result<(String, String, f64, f64, f64, f64)> {
        let mut lat_ns = "N".to_string();
        let mut lon_ew = "E".to_string();
        let mut latmin = 0.0;
        let mut lonmin = 0.0;
        let mut latsec = 0.0;
        let mut lonsec = 0.0;

        // Check for semicolon-separated format (e.g., "40.7128;-74.0060")
        if let Some(i) = self.pieces[0].find(';') {
            let piece = self.pieces.remove(0);
            self.latdeg = piece[..i].parse().unwrap_or(0.0);
            self.londeg = piece[i + 1..].parse().unwrap_or(0.0);
            self.coor = vec![self.latdeg.to_string(), self.londeg.to_string()];
        }
        // Degrees only format (e.g., "40 N 74 W")
        else if self.pieces.len() >= 4 && Self::is_coor(&self.pieces[1], &self.pieces[3]) {
            self.latdeg = self.parse_piece();
            lat_ns = self.pieces.remove(0);
            self.londeg = self.parse_piece();
            lon_ew = self.pieces.remove(0);
            self.coor = vec![
                self.latdeg.to_string(),
                lat_ns.clone(),
                self.londeg.to_string(),
                lon_ew.clone(),
            ];
        }
        // Degrees + minutes format (e.g., "40 30 N 74 0 W")
        else if self.pieces.len() >= 6 && Self::is_coor(&self.pieces[2], &self.pieces[5]) {
            self.latdeg = self.parse_piece();
            latmin = self.parse_piece();
            lat_ns = self.pieces.remove(0);
            self.londeg = self.parse_piece();
            lonmin = self.parse_piece();
            lon_ew = self.pieces.remove(0);
            self.coor = vec![
                self.latdeg.to_string(),
                latmin.to_string(),
                lat_ns.clone(),
                self.londeg.to_string(),
                lonmin.to_string(),
                lon_ew.clone(),
            ];
        }
        // Degrees + minutes + seconds format (e.g., "40 30 45 N 74 0 21 W")
        else if self.pieces.len() >= 8 && Self::is_coor(&self.pieces[3], &self.pieces[7]) {
            self.latdeg = self.parse_piece();
            latmin = self.parse_piece();
            latsec = self.parse_piece();
            lat_ns = self.pieces.remove(0);
            self.londeg = self.parse_piece();
            lonmin = self.parse_piece();
            lonsec = self.parse_piece();
            lon_ew = self.pieces.remove(0);
            self.coor = vec![
                self.latdeg.to_string(),
                latmin.to_string(),
                latsec.to_string(),
                lat_ns.clone(),
                self.londeg.to_string(),
                lonmin.to_string(),
                lonsec.to_string(),
                lon_ew.clone(),
            ];
        } else {
            return Err(anyhow!("Unrecognized format"));
        }

        Ok((lat_ns, lon_ew, latmin, lonmin, latsec, lonsec))
    }

    /// Validate coordinate ranges
    fn validate_ranges(&self, latmin: f64, lonmin: f64, latsec: f64, lonsec: f64) -> Result<()> {
        let valid_degree_range = |deg: f64, max: f64| (-max..=max).contains(&deg);
        let valid_minsec_range = |val: f64| (0.0..=60.0).contains(&val);

        if !valid_degree_range(self.latdeg, 90.0)
            || !valid_degree_range(self.londeg, 360.0)
            || !valid_minsec_range(latmin)
            || !valid_minsec_range(lonmin)
            || !valid_minsec_range(latsec)
            || !valid_minsec_range(lonsec)
        {
            return Err(anyhow!("Out of range"));
        }
        Ok(())
    }

    /// Convert parsed values to decimal degrees
    fn convert_to_decimal_degrees(
        &mut self,
        lat_ns: &str,
        lon_ew: &str,
        latmin: f64,
        lonmin: f64,
        latsec: f64,
        lonsec: f64,
    ) {
        let direction_factor = |dir: &str, neg_dir: &str| {
            if dir.eq_ignore_ascii_case(neg_dir) {
                -1.0
            } else {
                1.0
            }
        };

        let latfactor = direction_factor(lat_ns, "S");
        let lonfactor = direction_factor(lon_ew, "W");

        // Convert minutes and seconds to decimal
        let latmin_total = latmin + latsec / 60.0;
        let lonmin_total = lonmin + lonsec / 60.0;

        // Add or subtract based on sign of degrees
        self.latdeg += latmin_total.copysign(self.latdeg) / 60.0;
        self.londeg += lonmin_total.copysign(self.londeg) / 60.0;

        self.latdeg *= latfactor;
        self.londeg *= lonfactor;
    }

    /// Given decimal degrees, convert to minutes, seconds and direction
    pub fn make_minsec(deg: f64) -> MinSecResult {
        let (ns, ew) = if deg >= 0.0 { ("N", "E") } else { ("S", "W") };

        // Round to a suitable number of digits
        let deg_rounded = (deg * 1_000_000.0).round() / 1_000_000.0;
        let min = 60.0 * (deg_rounded.abs() - deg_rounded.abs().floor());
        let min_rounded = (min * 10_000.0).round() / 10_000.0;
        let sec = 60.0 * (min_rounded - min_rounded.floor());
        let sec_rounded = (sec * 100.0).round() / 100.0;

        MinSecResult {
            deg: deg_rounded,
            min: min_rounded,
            sec: sec_rounded,
            ns: ns.to_string(),
            ew: ew.to_string(),
        }
    }

    /// Given decimal degrees latitude and longitude, convert to string
    pub fn make_position(lat: f64, lon: f64) -> String {
        let latdms = Self::make_minsec(lat);
        let londms = Self::make_minsec(lon);

        let mut outlat = format!("{}°\u{00A0}", latdms.deg.abs() as i32);
        let mut outlon = format!("{}°\u{00A0}", londms.deg.abs() as i32);

        if latdms.min != 0.0 || londms.min != 0.0 || latdms.sec != 0.0 || londms.sec != 0.0 {
            outlat.push_str(&format!("{}′\u{00A0}", latdms.min as i32));
            outlon.push_str(&format!("{}′\u{00A0}", londms.min as i32));

            if latdms.sec != 0.0 || londms.sec != 0.0 {
                outlat.push_str(&format!("{}″\u{00A0}", latdms.sec));
                outlon.push_str(&format!("{}″\u{00A0}", londms.sec));
            }
        }

        format!("{}{} {}{}", outlat, latdms.ns, outlon, londms.ew)
    }

    /// Get the additional attributes in an associative array (HashMap)
    pub fn get_attr(&mut self) -> HashMap<String, String> {
        let mut attributes = HashMap::new();

        while let Some(s) = self.pieces.pop() {
            if let Some(i) = s.find(':').filter(|&i| i >= 1) {
                let attr = &s[..i];
                let mut val = &s[i + 1..];

                // Check for arguments in parentheses (e.g., "type:city(7000000)")
                if let Some((j, k)) = val.find('(').zip(val.find(')')).filter(|(j, k)| k > j) {
                    attributes.insert(format!("arg:{attr}"), val[j + 1..k].to_string());
                    val = &val[..j];
                }

                attributes.insert(attr.to_string(), val.to_string());
            } else if let Ok(num) = s.parse::<i32>() {
                // Bare number is treated as scale if not already set
                if num > 0 && !attributes.contains_key("scale") {
                    attributes.insert("scale".to_string(), num.to_string());
                }
            }
        }

        attributes
    }

    /// Check if strings represent valid N/S and E/W directions
    fn is_coor(ns: &str, ew: &str) -> bool {
        let ns = ns.to_uppercase();
        let ew = ew.to_uppercase();
        (ns == "N" || ns == "S") && (ew == "E" || ew == "W")
    }

    /// Get composite position in RFC2045 format
    pub fn get_position(&self) -> String {
        format!("{};{}", self.latdeg, self.londeg)
    }

    /// Produce markup suitable for use in page
    /// Use original content as much as possible
    pub fn get_markup(&self) -> Result<String> {
        let n = self.coor.len();

        if n == 0 {
            // Range is special case
            Ok(format!(
                "{} to {}",
                Self::make_position(self.latdeg_min, self.londeg_min),
                Self::make_position(self.latdeg_max, self.londeg_max)
            ))
        } else if n == 2 {
            Ok(format!("{};{}", self.coor[0], self.coor[1]))
        } else if n == 4 {
            Ok(format!(
                "{}°\u{00A0}{} {}°\u{00A0}{}",
                self.coor[0], self.coor[1], self.coor[2], self.coor[3]
            ))
        } else if n == 6 {
            Ok(format!(
                "{}°{}′\u{00A0}{} {}°{}′\u{00A0}{}",
                self.coor[0], self.coor[1], self.coor[2], self.coor[3], self.coor[4], self.coor[5]
            ))
        } else if n == 8 {
            Ok(format!(
                "{}°{}′{}″\u{00A0}{} {}°{}′{}″\u{00A0}{}",
                self.coor[0],
                self.coor[1],
                self.coor[2],
                self.coor[3],
                self.coor[4],
                self.coor[5],
                self.coor[6],
                self.coor[7]
            ))
        } else {
            Err(anyhow!("Invalid coordinate length"))
        }
    }

    pub const fn coor(&self) -> &Vec<String> {
        &self.coor
    }

    pub const fn pieces(&self) -> &Vec<String> {
        &self.pieces
    }

    pub const fn pieces_mut(&mut self) -> &mut Vec<String> {
        &mut self.pieces
    }
}

/// Result structure for make_minsec function
#[derive(Debug, Clone, Default)]
pub struct MinSecResult {
    pub deg: f64,
    pub min: f64,
    pub sec: f64,
    pub ns: String,
    pub ew: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_coordinates() {
        let geo = GeoParam::new("40.7128;-74.0060").unwrap();
        assert_eq!(geo.latdeg, 40.7128);
        assert_eq!(geo.londeg, -74.0060);
    }

    #[test]
    fn test_degrees_with_direction() {
        let geo = GeoParam::new("40 N 74 W").unwrap();
        assert_eq!(geo.latdeg, 40.0);
        assert_eq!(geo.londeg, -74.0);
    }

    #[test]
    fn test_degrees_minutes_seconds() {
        let geo = GeoParam::new("40 42 46 N 74 0 21 W").unwrap();
        assert!(geo.latdeg > 40.7 && geo.latdeg < 40.8);
        assert!(geo.londeg < -73.9 && geo.londeg > -74.1);
    }

    #[test]
    fn test_range() {
        let geo = GeoParam::new("40 N 74 W to 41 N 73 W").unwrap();
        assert_eq!(geo.latdeg_min, 40.0);
        assert_eq!(geo.latdeg_max, 41.0);
        assert_eq!(geo.londeg_min, -74.0);
        assert_eq!(geo.londeg_max, -73.0);
        assert_eq!(geo.latdeg, 40.5);
        assert_eq!(geo.londeg, -73.5);
    }

    #[test]
    fn test_make_position() {
        let position = GeoParam::make_position(40.7128, -74.0060);
        assert!(position.contains("40°"));
        assert!(position.contains("N"));
        assert!(position.contains("74°"));
        assert!(position.contains("W"));
    }

    #[test]
    fn test_out_of_range() {
        match GeoParam::new("91 N 180 E") {
            Ok(_geo) => panic!("This should have failed!"),
            Err(err) => assert_eq!(err.to_string(), "Out of range"),
        };
    }

    #[test]
    fn test_degrees_minutes() {
        // Test the 6-piece format (degrees + minutes)
        let geo = GeoParam::new("40 30 N 74 15 W").unwrap();
        assert_eq!(geo.latdeg, 40.5); // 40 + 30/60 = 40.5
        assert_eq!(geo.londeg, -74.25); // -(74 + 15/60) = -74.25
    }

    #[test]
    fn test_get_position() {
        let geo = GeoParam::new("40.5_N_74.5_W").unwrap();
        assert_eq!(geo.get_position(), "40.5;-74.5");
    }

    #[test]
    fn test_get_markup_semicolon() {
        let geo = GeoParam::new("40.5;-74.5").unwrap();
        let markup = geo.get_markup().unwrap();
        assert_eq!(markup, "40.5;-74.5");
    }

    #[test]
    fn test_get_markup_degrees() {
        let geo = GeoParam::new("40 N 74 W").unwrap();
        let markup = geo.get_markup().unwrap();
        assert!(markup.contains("40°"));
        assert!(markup.contains("N"));
        assert!(markup.contains("74°"));
        assert!(markup.contains("W"));
    }

    #[test]
    fn test_get_attr_with_type() {
        let mut geo = GeoParam::new("40_N_74_W_type:city").unwrap();
        let attr = geo.get_attr();
        assert_eq!(attr.get("type"), Some(&"city".to_string()));
    }

    #[test]
    fn test_get_attr_with_type_and_arg() {
        let mut geo = GeoParam::new("40_N_74_W_type:city(7000000)").unwrap();
        let attr = geo.get_attr();
        assert_eq!(attr.get("type"), Some(&"city".to_string()));
        assert_eq!(attr.get("arg:type"), Some(&"7000000".to_string()));
    }

    #[test]
    fn test_get_attr_with_scale() {
        let mut geo = GeoParam::new("40_N_74_W_100000").unwrap();
        let attr = geo.get_attr();
        assert_eq!(attr.get("scale"), Some(&"100000".to_string()));
    }

    #[test]
    fn test_get_attr_multiple() {
        let mut geo = GeoParam::new("40_N_74_W_type:city_region:US-NY_source:enwiki").unwrap();
        let attr = geo.get_attr();
        assert_eq!(attr.get("type"), Some(&"city".to_string()));
        assert_eq!(attr.get("region"), Some(&"US-NY".to_string()));
        assert_eq!(attr.get("source"), Some(&"enwiki".to_string()));
    }

    #[test]
    fn test_o_replacement() {
        // 'O' should be replaced with 'E' (German "Ost" for East)
        let geo = GeoParam::new("40_N_74_O").unwrap();
        assert_eq!(geo.londeg, 74.0); // Positive because O -> E (East)
    }

    #[test]
    fn test_region_code_preserved() {
        let mut geo = GeoParam::new("40_N_74_W_region:JO").unwrap();
        let attr = geo.get_attr();
        assert_eq!(attr.get("region"), Some(&"JO".to_string())); // Jordan, not Jersey

        let mut geo2 = GeoParam::new("40_N_74_W_region:CA-ON").unwrap();
        let attr2 = geo2.get_attr();
        assert_eq!(attr2.get("region"), Some(&"CA-ON".to_string())); // Ontario, not CA-EN
    }

    #[test]
    fn test_south_latitude() {
        let geo = GeoParam::new("35_S_149_E").unwrap();
        assert_eq!(geo.latdeg, -35.0);
        assert_eq!(geo.londeg, 149.0);
    }

    #[test]
    fn test_make_minsec() {
        let result = GeoParam::make_minsec(40.5);
        assert_eq!(result.deg, 40.5);
        assert_eq!(result.min as i32, 30);
        assert_eq!(result.ns, "N");
        assert_eq!(result.ew, "E");

        let result_neg = GeoParam::make_minsec(-74.25);
        assert_eq!(result_neg.deg, -74.25);
        assert_eq!(result_neg.min as i32, 15);
        assert_eq!(result_neg.ns, "S");
        assert_eq!(result_neg.ew, "W");
    }

    #[test]
    fn test_no_coordinates_error() {
        match GeoParam::new("") {
            Ok(_) => panic!("Should have failed"),
            Err(e) => assert_eq!(e.to_string(), "No coordinates provided"),
        }
    }

    #[test]
    fn test_unrecognized_format_error() {
        match GeoParam::new("invalid coordinates here") {
            Ok(_) => panic!("Should have failed"),
            Err(e) => assert_eq!(e.to_string(), "Unrecognized format"),
        }
    }
}
