use anyhow::{Result, anyhow};

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
        let mut geo = Self {
            ..Default::default()
        };

        // Replace 'O' with 'E' and underscores with spaces, then split
        let processed = param.replace('O', "E").replace('_', " ");
        geo.pieces = processed.split_whitespace().map(String::from).collect();

        geo.get_coor()?;

        geo.latdeg_min = geo.latdeg;
        geo.latdeg_max = geo.latdeg;
        geo.londeg_min = geo.londeg;
        geo.londeg_max = geo.londeg;

        if !geo.pieces.is_empty() && geo.pieces[0] == "to" {
            geo.pieces.remove(0);
            geo.get_coor()?;

            if geo.latdeg < geo.latdeg_max {
                geo.latdeg_min = geo.latdeg;
            } else {
                geo.latdeg_max = geo.latdeg;
            }

            if geo.londeg < geo.londeg_max {
                geo.londeg_min = geo.londeg;
            } else {
                geo.londeg_max = geo.londeg;
            }

            geo.latdeg = (geo.latdeg_max + geo.latdeg_min) / 2.0;
            geo.londeg = (geo.londeg_max + geo.londeg_min) / 2.0;
            geo.coor = Vec::new();
        }

        Ok(geo)
    }

    pub const fn latdeg(&self) -> f64 {
        self.latdeg
    }

    pub const fn londeg(&self) -> f64 {
        self.londeg
    }

    /// Get a set of coordinates from parameters
    fn get_coor(&mut self) -> Result<()> {
        if self.pieces.is_empty() {
            return Err(anyhow!("No coordinates provided"));
        }

        let mut lat_ns = "N".to_string();
        let mut lon_ew = "E".to_string();
        let mut latmin = 0.0;
        let mut lonmin = 0.0;
        let mut latsec = 0.0;
        let mut lonsec = 0.0;

        // Check for semicolon-separated format
        if let Some(i) = self.pieces[0].find(';') {
            let piece = self.pieces[0].clone();
            self.latdeg = piece[..i].parse().unwrap_or(0.0);
            self.londeg = piece[i + 1..].parse().unwrap_or(0.0);
            self.coor = vec![self.latdeg.to_string(), self.londeg.to_string()];
            self.pieces.remove(0);
        } else if self.pieces.len() >= 4 && Self::is_coor(&self.pieces[1], &self.pieces[3]) {
            // Check various coordinate formats
            self.latdeg = self.pieces.remove(0).parse().unwrap_or(0.0);
            lat_ns = self.pieces.remove(0);
            self.londeg = self.pieces.remove(0).parse().unwrap_or(0.0);
            lon_ew = self.pieces.remove(0);
            self.coor = vec![
                self.latdeg.to_string(),
                lat_ns.clone(),
                self.londeg.to_string(),
                lon_ew.clone(),
            ];
        } else if self.pieces.len() >= 6 && Self::is_coor(&self.pieces[2], &self.pieces[5]) {
            self.latdeg = self.pieces.remove(0).parse().unwrap_or(0.0);
            latmin = self.pieces.remove(0).parse().unwrap_or(0.0);
            lat_ns = self.pieces.remove(0);
            self.londeg = self.pieces.remove(0).parse().unwrap_or(0.0);
            lonmin = self.pieces.remove(0).parse().unwrap_or(0.0);
            lon_ew = self.pieces.remove(0);
            self.coor = vec![
                self.latdeg.to_string(),
                latmin.to_string(),
                lat_ns.clone(),
                self.londeg.to_string(),
                lonmin.to_string(),
                lon_ew.clone(),
            ];
        } else if self.pieces.len() >= 8 && Self::is_coor(&self.pieces[3], &self.pieces[7]) {
            self.latdeg = self.pieces.remove(0).parse().unwrap_or(0.0);
            latmin = self.pieces.remove(0).parse().unwrap_or(0.0);
            latsec = self.pieces.remove(0).parse().unwrap_or(0.0);
            lat_ns = self.pieces.remove(0);
            self.londeg = self.pieces.remove(0).parse().unwrap_or(0.0);
            lonmin = self.pieces.remove(0).parse().unwrap_or(0.0);
            lonsec = self.pieces.remove(0).parse().unwrap_or(0.0);
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

        // Validate ranges
        if self.latdeg > 90.0
            || self.latdeg < -90.0
            || self.londeg > 360.0
            || self.londeg < -360.0
            || !(0.0..=60.0).contains(&latmin)
            || !(0.0..=60.0).contains(&lonmin)
            || !(0.0..=60.0).contains(&latsec)
            || !(0.0..=60.0).contains(&lonsec)
        {
            return Err(anyhow!("Out of range"));
        }

        let latfactor = if lat_ns.to_uppercase() == "S" {
            -1.0
        } else {
            1.0
        };

        let lonfactor = if lon_ew.to_uppercase() == "W" {
            -1.0
        } else {
            1.0
        };

        // Convert to decimal degrees
        let latmin = latmin + latsec / 60.0;
        let lonmin = lonmin + lonsec / 60.0;

        if self.latdeg < 0.0 {
            self.latdeg -= latmin / 60.0;
        } else {
            self.latdeg += latmin / 60.0;
        }

        if self.londeg < 0.0 {
            self.londeg -= lonmin / 60.0;
        } else {
            self.londeg += lonmin / 60.0;
        }

        self.latdeg *= latfactor;
        self.londeg *= lonfactor;
        Ok(())
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
    pub fn get_attr(&mut self) -> std::collections::HashMap<String, String> {
        let mut attributes = std::collections::HashMap::new();

        while let Some(s) = self.pieces.pop() {
            if let Some(i) = s.find(':') {
                if i >= 1 {
                    let attr = &s[..i];
                    let mut val = &s[i + 1..];

                    // Check for arguments in parentheses
                    if let (Some(j), Some(k)) = (val.find('('), val.find(')'))
                        && k > j
                    {
                        attributes.insert(format!("arg:{}", attr), val[j + 1..k].to_string());
                        val = &val[..j];
                    }

                    attributes.insert(attr.to_string(), val.to_string());
                }
            } else if let Ok(num) = s.parse::<i32>()
                && num > 0
                && !attributes.contains_key("scale")
            {
                attributes.insert("scale".to_string(), num.to_string());
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
#[derive(Debug)]
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
}
