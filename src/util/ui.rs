pub fn ago(duration: chrono::Duration) -> String {
    if duration.num_seconds().abs() < 100 {
        format!("{}s", duration.num_seconds())
    } else if duration.num_minutes() < 100 {
        format!("{}m", duration.num_minutes())
    } else if duration.num_hours() < 100 {
        format!("{}h", duration.num_hours())
    } else if duration.num_days() < 100 {
        format!("{}d", duration.num_days())
    } else {
        format!("{}w", duration.num_weeks())
    }
}
