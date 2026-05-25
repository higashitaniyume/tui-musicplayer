use std::path::Path;
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct LyricLine {
    pub timestamp: Duration,
    pub text: String,
}

pub fn parse_lrc(path: &Path) -> Option<Vec<LyricLine>> {
    let lrc_path = path.with_extension("lrc");
    let content = std::fs::read_to_string(&lrc_path).ok()?;
    let mut lines: Vec<LyricLine> = Vec::new();

    for raw in content.lines() {
        let line = raw.trim();
        if line.is_empty() {
            continue;
        }
        let mut timestamps = Vec::new();
        let mut search_start = 0;
        while let Some(start) = line[search_start..].find('[') {
            let abs_start = search_start + start;
            if let Some(end) = line[abs_start..].find(']') {
                let abs_end = abs_start + end;
                let tag = &line[abs_start + 1..abs_end];
                if let Some(ts) = parse_timestamp(tag) {
                    timestamps.push(ts);
                    search_start = abs_end + 1;
                } else {
                    search_start = abs_end + 1;
                }
            } else {
                break;
            }
        }
        let text_start = line.rfind(']').map(|i| i + 1).unwrap_or(0);
        let text = line[text_start..].trim().to_string();
        if text.is_empty() {
            continue;
        }
        for ts in timestamps {
            if lines.iter().any(|l| l.timestamp == ts) {
                continue;
            }
            lines.push(LyricLine { timestamp: ts, text: text.clone() });
        }
    }

    lines.sort_by_key(|l| l.timestamp);
    if lines.is_empty() { None } else { Some(lines) }
}

fn parse_timestamp(s: &str) -> Option<Duration> {
    let colon = s.find(':')?;
    let minutes: u64 = s[..colon].parse().ok()?;
    let rest = &s[colon + 1..];
    let dot = rest.find('.')?;
    let seconds: u64 = rest[..dot].parse().ok()?;
    let frac_str = &rest[dot + 1..];
    let frac_len = frac_str.len().min(3);
    let frac: u64 = frac_str[..frac_len].parse().ok()?;
    let millis = frac * 10u64.pow(3 - frac_len as u32);
    Some(Duration::new(minutes * 60 + seconds, millis as u32 * 1_000_000))
}
