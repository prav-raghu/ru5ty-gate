use chrono::{DateTime, Days, NaiveTime, SecondsFormat, Utc};

pub struct DateUtil;

impl DateUtil {
    pub fn to_iso_string(date: DateTime<Utc>) -> String {
        date.to_rfc3339_opts(SecondsFormat::Millis, true)
    }

    pub fn parse_iso_date(value: &str) -> Option<DateTime<Utc>> {
        DateTime::parse_from_rfc3339(value)
            .ok()
            .map(|parsed| parsed.with_timezone(&Utc))
    }

    pub fn is_valid_iso_date(value: &str) -> bool {
        Self::parse_iso_date(value).is_some()
    }

    pub fn is_future(date: DateTime<Utc>) -> bool {
        date > Utc::now()
    }

    pub fn is_past(date: DateTime<Utc>) -> bool {
        date < Utc::now()
    }

    pub fn add_days(date: DateTime<Utc>, days: u64) -> Option<DateTime<Utc>> {
        date.checked_add_days(Days::new(days))
    }

    pub fn start_of_utc_day(date: DateTime<Utc>) -> DateTime<Utc> {
        date.date_naive().and_time(NaiveTime::MIN).and_utc()
    }

    pub fn end_of_utc_day(date: DateTime<Utc>) -> DateTime<Utc> {
        let end = NaiveTime::from_hms_milli_opt(23, 59, 59, 999).unwrap_or(NaiveTime::MIN);
        date.date_naive().and_time(end).and_utc()
    }
}
