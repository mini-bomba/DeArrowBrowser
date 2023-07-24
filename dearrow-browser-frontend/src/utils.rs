use chrono::{DateTime, Utc, NaiveDateTime};

const TIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

pub fn render_datetime(dt: DateTime<Utc>) -> String 
{
    format!("{}", dt.format(TIME_FORMAT))
}
pub fn render_naive_datetime(dt: NaiveDateTime) -> String 
{
    format!("{}", dt.format(TIME_FORMAT))
}
pub fn render_datetime_with_delta(dt: DateTime<Utc>) -> String
{
    format!("{} UTC ({} minutes ago)", dt.format(TIME_FORMAT), (Utc::now()-dt).num_minutes())
}
