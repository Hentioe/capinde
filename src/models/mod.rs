use crate::{err, errors::Error};

pub mod api;
pub mod params;
pub mod payload;

pub enum WorkingMode {
    Localized,
    Hosted,
}

impl std::str::FromStr for WorkingMode {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "localized" => Ok(WorkingMode::Localized),
            "hosted" => Ok(WorkingMode::Hosted),
            _ => err!("invalid work mode: {s}, only 'localized' or 'hosted' are allowed"),
        }
    }
}

impl std::fmt::Display for WorkingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WorkingMode::Localized => write!(f, "localized"),
            WorkingMode::Hosted => write!(f, "hosted"),
        }
    }
}
