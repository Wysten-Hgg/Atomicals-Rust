use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, AddAssign, Sub, SubAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Amount(pub u64);

impl Amount {
    pub const ZERO: Amount = Amount(0);
    pub const SATOSHI: Amount = Amount(1);
    pub const BITCOIN: Amount = Amount(100_000_000);

    pub fn from_sat(sats: u64) -> Self {
        Amount(sats)
    }

    pub fn from_btc(btc: f64) -> Result<Self, &'static str> {
        if !btc.is_finite() {
            return Err("Invalid bitcoin amount: not finite");
        }
        if btc < 0.0 {
            return Err("Invalid bitcoin amount: negative");
        }
        let sats = (btc * 100_000_000.0).round();
        if sats > u64::MAX as f64 {
            return Err("Invalid bitcoin amount: overflow");
        }
        Ok(Amount(sats as u64))
    }

    pub fn to_sat(self) -> u64 {
        self.0
    }

    pub fn to_btc(self) -> f64 {
        self.0 as f64 / 100_000_000.0
    }
}

impl Add for Amount {
    type Output = Amount;

    fn add(self, other: Amount) -> Amount {
        Amount(self.0.checked_add(other.0).expect("Amount overflow"))
    }
}

impl AddAssign for Amount {
    fn add_assign(&mut self, other: Amount) {
        *self = *self + other;
    }
}

impl Sub for Amount {
    type Output = Amount;

    fn sub(self, other: Amount) -> Amount {
        Amount(self.0.checked_sub(other.0).expect("Amount underflow"))
    }
}

impl SubAssign for Amount {
    fn sub_assign(&mut self, other: Amount) {
        *self = *self - other;
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} sats", self.0)
    }
}
