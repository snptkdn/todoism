use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, TimeZone, Utc, Weekday};
use anyhow::{anyhow, Result};

pub fn parse_duration(input: &str) -> Result<Duration> {
    let input = input.trim();
    if input.is_empty() {
        return Err(anyhow!("Empty duration string"));
    }

    let len = input.len();
    let (num_str, unit) = input.split_at(len - 1);
    
    let num: i64 = num_str.parse().map_err(|_| anyhow!("Invalid duration number"))?;
    
    match unit.to_lowercase().as_str() {
        "m" => Ok(Duration::minutes(num)),
        "h" => Ok(Duration::hours(num)),
        "d" => Ok(Duration::days(num)),
        "w" => Ok(Duration::weeks(num)),
        _ => Err(anyhow!("Unknown duration unit: {}", unit)),
    }
}

pub fn parse_human_date(input: &str) -> Result<DateTime<Utc>> {
    let now = Local::now(); // Use local time for calculation relative to user
    let today = now.date_naive();
    
    // 1. Reserved keywords
    match input.to_lowercase().as_str() {
        "today" | "tod" => return end_of_day(today),
        "tomorrow" | "tom" => return end_of_day(today + Duration::days(1)),
        "eow" => {
            // End of week (Sunday)
            let days_to_sunday = Weekday::Sun.num_days_from_sunday() as i64 - today.weekday().num_days_from_sunday() as i64;
            let target = if days_to_sunday >= 0 {
                today + Duration::days(days_to_sunday)
            } else {
                today + Duration::days(days_to_sunday + 7)
            };
             return end_of_day(target);
        }
        "eom" => {
             // End of month
             let next_month = if today.month() == 12 {
                 NaiveDate::from_ymd_opt(today.year() + 1, 1, 1).unwrap()
             } else {
                 NaiveDate::from_ymd_opt(today.year(), today.month() + 1, 1).unwrap()
             };
             return end_of_day(next_month - Duration::days(1));
        }
        _ => {}
    }

    // 2. Relative format (+Nd, +Nw, +Nm)
    if input.starts_with('+') {
        let (num_str, unit) = input[1..].split_at(input.len() - 2);
        let count: i64 = num_str.parse().map_err(|_| anyhow!("Invalid relative format"))?;
        
        let target = match unit {
            "d" => today + Duration::days(count),
            "w" => today + Duration::weeks(count),
            "m" => {
                // Simplified month addition
                 let new_month = today.month() as i64 + count;
                 let new_year = today.year() as i64 + (new_month - 1) / 12;
                 let new_month_val = ((new_month - 1) % 12 + 1) as u32;
                 NaiveDate::from_ymd_opt(new_year as i32, new_month_val, today.day()).unwrap_or(
                     // Fallback to end of month if day doesn't exist (e.g. Jan 31 + 1m -> Feb 28)
                     if new_month_val == 12 {
                         NaiveDate::from_ymd_opt(new_year as i32 + 1, 1, 1).unwrap() - Duration::days(1)
                     } else {
                         NaiveDate::from_ymd_opt(new_year as i32, new_month_val + 1, 1).unwrap() - Duration::days(1)
                     }
                 )
            },
            _ => return Err(anyhow!("Unknown unit in relative time: {}", unit)),
        };
        return end_of_day(target);
    }

    // 3. Weekday format (fri, 2:fri)
    if let Some((count, day_str)) = parse_weekday_token(input) {
        if let Ok(target_weekday) = parse_weekday_str(day_str) {
            let mut days_needed = target_weekday.num_days_from_sunday() as i64 - today.weekday().num_days_from_sunday() as i64;
            if days_needed <= 0 {
                days_needed += 7;
            }
            // count = 1 means next X (e.g. next Friday). count = 2 means the one after that.
            // so we add (count - 1) weeks.
            days_needed += (count - 1) * 7;
            
            return end_of_day(today + Duration::days(days_needed));
        }
    }
    
    // 4. Fallback to standard formats
     if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(input, "%Y-%m-%d %H:%M:%S") {
        return Ok(Local.from_local_datetime(&dt).unwrap().with_timezone(&Utc));
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return end_of_day(d);
    }

    Err(anyhow!("Could not parse date: {}", input))
}

fn end_of_day(date: NaiveDate) -> Result<DateTime<Utc>> {
    let local_dt = date.and_hms_opt(23, 59, 59).unwrap();
    // Convert Local to UTC
    Ok(Local.from_local_datetime(&local_dt).unwrap().with_timezone(&Utc))
}

fn parse_weekday_token(input: &str) -> Option<(i64, &str)> {
    if input.contains(':') {
        let parts: Vec<&str> = input.split(':').collect();
        if parts.len() == 2 {
            if let Ok(count) = parts[0].parse::<i64>() {
                return Some((count, parts[1]));
            }
        }
    } else {
        // Just "fri" means 1:fri
        return Some((1, input));
    }
    None
}

fn parse_weekday_str(s: &str) -> Result<Weekday> {
    match s.to_lowercase().as_str() {
        "mon" | "monday" => Ok(Weekday::Mon),
        "tue" | "tuesday" => Ok(Weekday::Tue),
        "wed" | "wednesday" => Ok(Weekday::Wed),
        "thu" | "thursday" => Ok(Weekday::Thu),
        "fri" | "friday" => Ok(Weekday::Fri),
        "sat" | "saturday" => Ok(Weekday::Sat),
        "sun" | "sunday" => Ok(Weekday::Sun),
        _ => Err(anyhow!("Invalid weekday")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Note: These tests depend on "now". For robust testing we should mock time,
    // but for this prototype, we'll just test the parsing logic logic relative to a known anchor if we could inject it,
    // or just smoke test that it doesn't crash.
    // Actually, let's skip "now" dependent logic tests for a second or trust the logic.
    // Or better, testing helper.
    
    #[test]
    fn test_parse_weekday_token() {
        assert_eq!(parse_weekday_token("fri"), Some((1, "fri")));
        assert_eq!(parse_weekday_token("2:fri"), Some((2, "fri")));
        assert_eq!(parse_weekday_token("10:mon"), Some((10, "mon")));
        assert_eq!(parse_weekday_token("invalid"), Some((1, "invalid"))); // will fail later at weekday parse
    }
}
