//! Task schedules, expressed in local time of the task's timezone.

use chrono::{DateTime, Datelike, Days, LocalResult, NaiveDateTime, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Schedule {
    Once {
        at: NaiveDateTime,
    },
    /// `weekdays` uses ISO numbering (1 = Monday … 7 = Sunday); empty means every day.
    Daily {
        time: NaiveTime,
        #[serde(default)]
        weekdays: Vec<u8>,
    },
}

const WEEKDAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

/// Local → UTC. Ambiguous times (DST fall-back) take the earliest instant; nonexistent ones
/// (DST spring-forward gap) yield `None`.
fn to_utc(tz: Tz, local: NaiveDateTime) -> Option<DateTime<Utc>> {
    match tz.from_local_datetime(&local) {
        LocalResult::Single(t) => Some(t.with_timezone(&Utc)),
        LocalResult::Ambiguous(earliest, _) => Some(earliest.with_timezone(&Utc)),
        LocalResult::None => None,
    }
}

impl Schedule {
    /// The first run strictly after `after`, or `None` when there is none.
    pub fn next_run_after(&self, tz: Tz, after: DateTime<Utc>) -> Option<DateTime<Utc>> {
        match self {
            Schedule::Once { at } => to_utc(tz, *at).filter(|t| *t > after),
            Schedule::Daily { time, weekdays } => {
                let start = after.with_timezone(&tz).date_naive();
                (0..8)
                    .filter_map(|d| start.checked_add_days(Days::new(d)))
                    .filter(|date| {
                        weekdays.is_empty() || weekdays.contains(&(date.weekday().number_from_monday() as u8))
                    })
                    .filter_map(|date| to_utc(tz, date.and_time(*time)))
                    .find(|t| *t > after)
            }
        }
    }

    pub fn describe(&self) -> String {
        match self {
            Schedule::Once { at } => format!("once at {}", at.format("%Y-%m-%d %H:%M")),
            Schedule::Daily { time, weekdays } if weekdays.is_empty() => {
                format!("daily at {}", time.format("%H:%M"))
            }
            Schedule::Daily { time, weekdays } => {
                let days: Vec<&str> = weekdays
                    .iter()
                    .filter_map(|d| WEEKDAY_NAMES.get(usize::from(*d).checked_sub(1)?).copied())
                    .collect();
                format!("daily at {} ({})", time.format("%H:%M"), days.join(", "))
            }
        }
    }
}

pub fn parse_local_datetime(s: &str) -> Option<NaiveDateTime> {
    ["%Y-%m-%d %H:%M", "%Y-%m-%dT%H:%M", "%Y-%m-%d %H:%M:%S", "%Y-%m-%dT%H:%M:%S"]
        .iter()
        .find_map(|f| NaiveDateTime::parse_from_str(s.trim(), f).ok())
}

pub fn parse_local_time(s: &str) -> Option<NaiveTime> {
    ["%H:%M", "%H:%M:%S"]
        .iter()
        .find_map(|f| NaiveTime::parse_from_str(s.trim(), f).ok())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono_tz::{America::New_York, Asia::Jakarta};

    fn jakarta(s: &str) -> DateTime<Utc> {
        to_utc(Jakarta, parse_local_datetime(s).unwrap()).unwrap()
    }

    fn daily(time: &str, weekdays: Vec<u8>) -> Schedule {
        Schedule::Daily { time: parse_local_time(time).unwrap(), weekdays }
    }

    #[test]
    fn once_in_the_past_has_no_next_run() {
        let s = Schedule::Once { at: parse_local_datetime("2026-11-10 08:00").unwrap() };
        assert_eq!(s.next_run_after(Jakarta, jakarta("2026-11-10 07:59")), Some(jakarta("2026-11-10 08:00")));
        assert_eq!(s.next_run_after(Jakarta, jakarta("2026-11-10 08:00")), None);
    }

    #[test]
    fn daily_runs_today_until_the_time_then_tomorrow() {
        let s = daily("08:00", vec![]);
        assert_eq!(s.next_run_after(Jakarta, jakarta("2026-11-10 07:59")), Some(jakarta("2026-11-10 08:00")));
        assert_eq!(s.next_run_after(Jakarta, jakarta("2026-11-10 08:00")), Some(jakarta("2026-11-11 08:00")));
    }

    #[test]
    fn weekday_filter_skips_to_next_allowed_day() {
        // 2026-11-10 is a Tuesday.
        let s = daily("08:00", vec![1]);
        assert_eq!(s.next_run_after(Jakarta, jakarta("2026-11-10 07:00")), Some(jakarta("2026-11-16 08:00")));
    }

    #[test]
    fn dst_gap_skips_to_next_day() {
        // 2026-03-08 02:30 does not exist in New York (clocks jump 02:00 → 03:00).
        let s = daily("02:30", vec![]);
        let after = to_utc(New_York, parse_local_datetime("2026-03-08 00:00").unwrap()).unwrap();
        let expected = to_utc(New_York, parse_local_datetime("2026-03-09 02:30").unwrap()).unwrap();
        assert_eq!(s.next_run_after(New_York, after), Some(expected));
    }

    #[test]
    fn describe_lists_weekdays() {
        assert_eq!(daily("08:00", vec![]).describe(), "daily at 08:00");
        assert_eq!(daily("08:00", vec![1, 3]).describe(), "daily at 08:00 (Mon, Wed)");
        let once = Schedule::Once { at: parse_local_datetime("2026-11-10 08:00").unwrap() };
        assert_eq!(once.describe(), "once at 2026-11-10 08:00");
    }

    #[test]
    fn parse_helpers_accept_and_reject() {
        assert!(parse_local_datetime("2026-11-10 08:00").is_some());
        assert!(parse_local_datetime("2026-11-10T08:00:30").is_some());
        assert!(parse_local_datetime("10/11/2026 08:00").is_none());
        assert!(parse_local_datetime("2026-11-10").is_none());
        assert!(parse_local_time("08:00").is_some());
        assert!(parse_local_time("08:00:15").is_some());
        assert!(parse_local_time("25:00").is_none());
        assert!(parse_local_time("8am").is_none());
    }

    #[test]
    fn serde_shape_is_tagged() {
        let json = serde_json::to_value(daily("08:00", vec![1])).unwrap();
        assert_eq!(json, serde_json::json!({"type": "daily", "time": "08:00:00", "weekdays": [1]}));
        let back: Schedule = serde_json::from_value(serde_json::json!({"type": "daily", "time": "08:00:00"})).unwrap();
        assert_eq!(back, daily("08:00", vec![]));
    }
}
