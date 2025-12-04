use chrono::{DateTime, Duration, Utc, TimeZone, Datelike, Timelike};
use std::time::{SystemTime, UNIX_EPOCH};

/// Time utility functions
pub struct TimeUtils;

impl TimeUtils {
    /// Get current timestamp (seconds)
    pub fn current_timestamp() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    }
    
    /// Get current timestamp (milliseconds)
    pub fn current_timestamp_millis() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
    }
    
    /// Get current timestamp (microseconds)
    pub fn current_timestamp_micros() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_micros() as u64
    }
    
    /// Create DateTime from seconds timestamp
    pub fn from_timestamp(timestamp: i64) -> Option<DateTime<Utc>> {
        Utc.timestamp_opt(timestamp, 0).single()
    }
    
    /// Create DateTime from milliseconds timestamp
    pub fn from_timestamp_millis(timestamp: i64) -> Option<DateTime<Utc>> {
        let seconds = timestamp / 1000;
        let nanos = ((timestamp % 1000) * 1_000_000) as u32;
        Utc.timestamp_opt(seconds, nanos).single()
    }
    
    /// Get time difference (seconds)
    pub fn time_diff_seconds(time1: DateTime<Utc>, time2: DateTime<Utc>) -> i64 {
        (time2 - time1).num_seconds()
    }
    
    /// Get time difference (minutes)
    pub fn time_diff_minutes(time1: DateTime<Utc>, time2: DateTime<Utc>) -> i64 {
        (time2 - time1).num_minutes()
    }
    
    /// Get time difference (hours)
    pub fn time_diff_hours(time1: DateTime<Utc>, time2: DateTime<Utc>) -> i64 {
        (time2 - time1).num_hours()
    }
    
    /// Get time difference (days)
    pub fn time_diff_days(time1: DateTime<Utc>, time2: DateTime<Utc>) -> i64 {
        (time2 - time1).num_days()
    }
    
    /// Format time difference
    pub fn format_time_diff(time1: DateTime<Utc>, time2: DateTime<Utc>) -> String {
        let diff = time2 - time1;
        
        if diff.num_days() > 0 {
            format!("{} days", diff.num_days())
        } else if diff.num_hours() > 0 {
            format!("{} hours", diff.num_hours())
        } else if diff.num_minutes() > 0 {
            format!("{} minutes", diff.num_minutes())
        } else {
            format!("{} seconds", diff.num_seconds())
        }
    }
    
    /// Check whether a time is expired
    pub fn is_expired(timestamp: DateTime<Utc>, expiry_duration: Duration) -> bool {
        Utc::now() > timestamp + expiry_duration
    }
    
    /// Get expiry time
    pub fn get_expiry_time(duration: Duration) -> DateTime<Utc> {
        Utc::now() + duration
    }
    
    /// Check whether a time is within a range
    pub fn is_in_time_range(
        time: DateTime<Utc>,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> bool {
        time >= start && time <= end
    }
    
    /// Get the start time of a window
    pub fn get_window_start(time: DateTime<Utc>, window_duration: Duration) -> DateTime<Utc> {
        let timestamp = time.timestamp();
        let window_seconds = window_duration.num_seconds();
        let window_start_timestamp = (timestamp / window_seconds) * window_seconds;
        Utc.timestamp_opt(window_start_timestamp, 0).single().unwrap_or(Utc::now())
    }
    
    /// Get the end time of a window
    pub fn get_window_end(time: DateTime<Utc>, window_duration: Duration) -> DateTime<Utc> {
        Self::get_window_start(time, window_duration) + window_duration
    }
    
    /// Check whether two times are in the same window
    pub fn is_same_window(
        time1: DateTime<Utc>,
        time2: DateTime<Utc>,
        window_duration: Duration,
    ) -> bool {
        Self::get_window_start(time1, window_duration) == Self::get_window_start(time2, window_duration)
    }
    
    /// Get window index
    pub fn get_window_index(time: DateTime<Utc>, window_duration: Duration) -> i64 {
        let timestamp = time.timestamp();
        let window_seconds = window_duration.num_seconds();
        timestamp / window_seconds
    }
    
    /// Get start time from window index
    pub fn from_window_index(index: i64, window_duration: Duration) -> DateTime<Utc> {
        let timestamp = index * window_duration.num_seconds();
        Utc.timestamp_opt(timestamp, 0).single().unwrap_or(Utc::now())
    }
    
    /// Get the current window start time
    pub fn current_window_start(window_duration: Duration) -> DateTime<Utc> {
        Self::get_window_start(Utc::now(), window_duration)
    }
    
    /// Get the current window end time
    pub fn current_window_end(window_duration: Duration) -> DateTime<Utc> {
        Self::get_window_end(Utc::now(), window_duration)
    }
    
    /// Get the next window start time
    pub fn next_window_start(window_duration: Duration) -> DateTime<Utc> {
        Self::current_window_start(window_duration) + window_duration
    }
    
    /// Get the previous window start time
    pub fn previous_window_start(window_duration: Duration) -> DateTime<Utc> {
        Self::current_window_start(window_duration) - window_duration
    }
    
    /// Count number of windows in a range
    pub fn count_windows(
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        window_duration: Duration,
    ) -> i64 {
        let start_index = Self::get_window_index(start_time, window_duration);
        let end_index = Self::get_window_index(end_time, window_duration);
        end_index - start_index + 1
    }
    
    /// Get a list of window start times
    pub fn get_window_list(
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        window_duration: Duration,
    ) -> Vec<DateTime<Utc>> {
        let mut windows = Vec::new();
        let mut current = Self::get_window_start(start_time, window_duration);
        let end = Self::get_window_end(end_time, window_duration);
        
        while current <= end {
            windows.push(current);
            current += window_duration;
        }
        
        windows
    }
    
    /// Check whether within business hours (Mon-Fri, 9:00-17:00 UTC)
    pub fn is_business_hours(time: DateTime<Utc>) -> bool {
        let weekday = time.weekday().num_days_from_monday();
        let hour = time.hour();
        
        // Monday to Friday (0-4)
        weekday < 5 && hour >= 9 && hour < 17
    }
    
    /// Get the next business day
    pub fn next_business_day(time: DateTime<Utc>) -> DateTime<Utc> {
        let mut next = time + Duration::days(1);
        
        while !Self::is_business_day(next) {
            next += Duration::days(1);
        }
        
        next
    }
    
    /// Get the previous business day
    pub fn previous_business_day(time: DateTime<Utc>) -> DateTime<Utc> {
        let mut prev = time - Duration::days(1);
        
        while !Self::is_business_day(prev) {
            prev -= Duration::days(1);
        }
        
        prev
    }
    
    /// Check whether it is a business day
    pub fn is_business_day(time: DateTime<Utc>) -> bool {
        let weekday = time.weekday().num_days_from_monday();
        weekday < 5 // Monday to Friday
    }
    
    /// Count business days in range
    pub fn count_business_days(
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> i64 {
        let mut count = 0;
        let mut current = start_time.date();
        let end = end_time.date();
        
        while current <= end {
            if Self::is_business_day(current.and_hms(0, 0, 0)) {
                count += 1;
            }
            current = current.succ();
        }
        
        count
    }
    
    /// Get business days within a time range
    pub fn get_business_days(
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
    ) -> Vec<DateTime<Utc>> {
        let mut business_days = Vec::new();
        let mut current = start_time.date();
        let end = end_time.date();
        
        while current <= end {
            if Self::is_business_day(current.and_hms(0, 0, 0)) {
                business_days.push(current.and_hms(9, 0, 0)); // 9:00 AM
            }
            current = current.succ();
        }
        
        business_days
    }
}

/// Time constants
pub mod constants {
    use chrono::Duration;
    
    pub const SECOND: Duration = Duration::seconds(1);
    pub const MINUTE: Duration = Duration::minutes(1);
    pub const HOUR: Duration = Duration::hours(1);
    pub const DAY: Duration = Duration::days(1);
    pub const WEEK: Duration = Duration::weeks(1);
    pub const MONTH: Duration = Duration::days(30);
    pub const YEAR: Duration = Duration::days(365);
    
    pub const BUSINESS_HOURS_START: u32 = 9;
    pub const BUSINESS_HOURS_END: u32 = 17;
    pub const BUSINESS_DAYS: [u32; 5] = [0, 1, 2, 3, 4]; // Monday to Friday
}

/// Time formatting utilities
pub struct TimeFormatUtils;

impl TimeFormatUtils {
    /// Format relative time
    pub fn format_relative_time(time: DateTime<Utc>) -> String {
        let now = Utc::now();
        let diff = now - time;
        
        if diff.num_seconds() < 60 {
            "just now".to_string()
        } else if diff.num_minutes() < 60 {
            format!("{} minutes ago", diff.num_minutes())
        } else if diff.num_hours() < 24 {
            format!("{} hours ago", diff.num_hours())
        } else if diff.num_days() < 7 {
            format!("{} days ago", diff.num_days())
        } else if diff.num_weeks() < 52 {
            format!("{} weeks ago", diff.num_weeks())
        } else {
            format!("{} years ago", diff.num_days() / 365)
        }
    }
    
    /// Format duration
    pub fn format_duration(duration: Duration) -> String {
        if duration.num_days() > 0 {
            format!("{}d {}h {}m", duration.num_days(), duration.num_hours() % 24, duration.num_minutes() % 60)
        } else if duration.num_hours() > 0 {
            format!("{}h {}m", duration.num_hours(), duration.num_minutes() % 60)
        } else if duration.num_minutes() > 0 {
            format!("{}m {}s", duration.num_minutes(), duration.num_seconds() % 60)
        } else {
            format!("{}s", duration.num_seconds())
        }
    }
    
    /// Format time range
    pub fn format_time_range(start: DateTime<Utc>, end: DateTime<Utc>) -> String {
        if start.date() == end.date() {
            format!(
                "{} {} - {}",
                start.format("%Y-%m-%d"),
                start.format("%H:%M"),
                end.format("%H:%M")
            )
        } else {
            format!(
                "{} - {}",
                start.format("%Y-%m-%d %H:%M"),
                end.format("%Y-%m-%d %H:%M")
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_current_timestamp() {
        let timestamp = TimeUtils::current_timestamp();
        assert!(timestamp > 0);
    }
    
    #[test]
    fn test_time_diff() {
        let time1 = Utc::now();
        let time2 = time1 + Duration::hours(2);
        
        assert_eq!(TimeUtils::time_diff_hours(time1, time2), 2);
    }
    
    #[test]
    fn test_window_calculation() {
        let now = Utc::now();
        let window_duration = Duration::hours(1);
        
        let window_start = TimeUtils::get_window_start(now, window_duration);
        let window_end = TimeUtils::get_window_end(now, window_duration);
        
        assert!(window_start <= now);
        assert!(window_end > now);
        assert_eq!(window_end - window_start, window_duration);
    }
    
    #[test]
    fn test_business_day() {
        let monday = Utc.ymd(2024, 1, 1).and_hms(10, 0, 0); // Monday
        let saturday = Utc.ymd(2024, 1, 6).and_hms(10, 0, 0); // Saturday
        
        assert!(TimeUtils::is_business_day(monday));
        assert!(!TimeUtils::is_business_day(saturday));
    }
}
