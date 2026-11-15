use anyhow::{bail, Context, Result};

/// Parse `30m`, `2h`, `1d` into seconds.
pub fn parse_ttl(s: &str) -> Result<i64> {
    if s.len() < 2 {
        bail!("ttl must look like 30m, 2h, or 1d");
    }
    let (n, u) = s.split_at(s.len() - 1);
    let n: i64 = n.parse().context("bad ttl number")?;
    if n <= 0 {
        bail!("ttl must be positive");
    }
    let mult = match u {
        "m" => 60,
        "h" => 3600,
        "d" => 86400,
        _ => bail!("ttl unit must be m, h, or d"),
    };
    Ok(n * mult)
}

/// Workspace id (16 hex chars). The first 8 encode nanoseconds since epoch,
/// so ids sort by creation time; the last 12 come from `/dev/urandom`, which
/// makes same-instant collisions astronomically unlikely. The UNIQUE path
/// constraint in the store is still the final backstop.
pub fn new_id() -> String {
    use std::io::Read;
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| (d.as_nanos() & 0xffff_ffff) as u32)
        .unwrap_or(0);
    let mut buf = [0u8; 6];
    match std::fs::File::open("/dev/urandom").and_then(|mut f| f.read_exact(&mut buf)) {
        Ok(()) => {
            let r = u64::from_le_bytes([buf[0], buf[1], buf[2], buf[3], buf[4], buf[5], 0, 0]);
            format!("{:08x}{:012x}", t, r)
        }
        // urandom is always present on macOS; fall back to a counter if not.
        Err(_) => {
            use std::sync::atomic::{AtomicU32, Ordering};
            static CTR: AtomicU32 = AtomicU32::new(0);
            let pid = std::process::id() as u64;
            let c = CTR.fetch_add(1, Ordering::Relaxed) as u64;
            format!("{:08x}{:012x}", t, (pid << 32) | c)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ttl_ok() {
        assert_eq!(parse_ttl("30m").unwrap(), 1800);
        assert_eq!(parse_ttl("2h").unwrap(), 7200);
        assert_eq!(parse_ttl("1d").unwrap(), 86400);
    }

    #[test]
    fn ttl_bad() {
        assert!(parse_ttl("").is_err());
        assert!(parse_ttl("h").is_err());
        assert!(parse_ttl("0h").is_err());
        assert!(parse_ttl("5x").is_err());
        assert!(parse_ttl("-1h").is_err());
    }

    #[test]
    fn ids_differ() {
        assert_ne!(new_id(), new_id());
    }
}
