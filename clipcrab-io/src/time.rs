pub fn parse_time(s: &str) -> Option<i64> {
    let parts = s.trim().split(':').collect::<Vec<&str>>();
    Some(match parts[..] {
        [hh, mm, ss, ..] => {
            hh.parse::<i64>().ok()? * 3600 + mm.parse::<i64>().ok()? * 60 + ss.parse::<i64>().ok()?
        }
        [mm, ss, ..] => mm.parse::<i64>().ok()? * 60 + ss.parse::<i64>().ok()?,
        _ => s.parse::<i64>().ok()?
    } * 1_000_000)
}