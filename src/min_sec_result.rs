/// Result structure for make_minsec function
#[derive(Debug, Clone, Default)]
pub struct MinSecResult {
    deg: f64,
    min: f64,
    sec: f64,
    ns: String,
    ew: String,
}

impl MinSecResult {
    /// Given decimal degrees, convert to minutes, seconds and direction
    pub fn new(deg: f64) -> Self {
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

    pub const fn deg(&self) -> f64 {
        self.deg
    }

    pub const fn min(&self) -> f64 {
        self.min
    }

    pub const fn sec(&self) -> f64 {
        self.sec
    }

    pub fn ns(&self) -> &str {
        &self.ns
    }

    pub fn ew(&self) -> &str {
        &self.ew
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new() {
        let result = MinSecResult::new(40.5);
        assert_eq!(result.deg(), 40.5);
        assert_eq!(result.min() as i32, 30);
        assert_eq!(result.ns(), "N");
        assert_eq!(result.ew(), "E");

        let result_neg = MinSecResult::new(-74.25);
        assert_eq!(result_neg.deg(), -74.25);
        assert_eq!(result_neg.min() as i32, 15);
        assert_eq!(result_neg.ns(), "S");
        assert_eq!(result_neg.ew(), "W");
    }
}
