use chrono::{DateTime, LocalResult, TimeZone, Utc};
use serde::Deserialize;

#[derive(Copy, Clone, Debug, strum::EnumString, strum::IntoStaticStr)]
pub enum Granularity {
    OneMin,
    FiveMin,
    FifteenMin,
    OneHour,
    SixHours,
    OneDay,
}

impl Granularity {
    pub(super) fn http_query_param(&self) -> &'static str {
        match self {
            Self::OneMin => "60",
            Self::FiveMin => "300",
            Self::FifteenMin => "900",
            Self::OneHour => "3600",
            Self::SixHours => "21600",
            Self::OneDay => "86400",
        }
    }
}

#[derive(Deserialize, Debug)]
#[serde(try_from = "CandleRepr")]
pub struct Candle {
    pub time: DateTime<Utc>,
    pub low: f64,
    pub high: f64,
    pub open: f64,
    pub close: f64,
    pub volume: f64,
}

impl TryFrom<CandleRepr> for Candle {
    type Error = &'static str;

    fn try_from(value: CandleRepr) -> Result<Self, Self::Error> {
        Ok(Self {
            time: match Utc.timestamp_opt(value.0, 0) {
                LocalResult::Single(time) => time,
                _ => return Err("invalid timestamp"),
            },
            low: value.1,
            high: value.2,
            open: value.3,
            close: value.4,
            volume: value.5,
        })
    }
}

#[derive(Deserialize)]
struct CandleRepr(i64, f64, f64, f64, f64, f64);
