// SPDX-License-Identifier: Apache-2.0
//! Author photos from Gravatar, fetched here (not by the webview) so the CSP stays closed.
//! Hits are cached 30 days, misses (404) 7 days; a network error is never cached.

use std::path::Path;

const HIT_TTL: u64 = 30 * 86_400;
const MISS_TTL: u64 = 7 * 86_400;

fn normalize(email: &str) -> String {
    email.trim().to_lowercase()
}

/// `d=404` makes Gravatar answer 404 for authors without a photo, so the UI can show initials.
pub fn gravatar_url(email: &str, size: u32) -> String {
    format!("https://www.gravatar.com/avatar/{:x}?s={size}&d=404", md5::compute(normalize(email)))
}

/// Cache file layout: byte 0 = 1 (photo) or 0 (known miss), bytes 1..9 = fetch time (secs, LE), then the image.
fn read_entry(path: &Path) -> Option<(bool, u64, Vec<u8>)> {
    let b = std::fs::read(path).ok()?;
    let ts = u64::from_le_bytes(b.get(1..9)?.try_into().ok()?);
    Some((*b.first()? == 1, ts, b[9..].to_vec()))
}

fn write_entry(path: &Path, hit: bool, now: u64, img: &[u8]) {
    let mut b = vec![u8::from(hit)];
    b.extend_from_slice(&now.to_le_bytes());
    b.extend_from_slice(img);
    let _ = std::fs::create_dir_all(path.parent().unwrap_or(Path::new(".")));
    let _ = std::fs::write(path, b);
}

/// Photo for `email`: from the disk cache while fresh, else `fetch` (`Ok(None)` = no photo, `Err` = network trouble).
pub fn lookup(dir: &Path, email: &str, now: u64, fetch: impl Fn(&str) -> Result<Option<Vec<u8>>, ()>) -> Option<Vec<u8>> {
    let email = normalize(email);
    if email.is_empty() {
        return None;
    }
    let path = dir.join(format!("{:x}.avatar", md5::compute(&email)));
    let cached = read_entry(&path);
    if let Some((hit, ts, img)) = &cached {
        let ttl = if *hit { HIT_TTL } else { MISS_TTL };
        if now.saturating_sub(*ts) < ttl {
            return hit.then(|| img.clone());
        }
    }
    match fetch(&gravatar_url(&email, 64)) {
        Ok(Some(img)) => {
            write_entry(&path, true, now, &img);
            Some(img)
        }
        Ok(None) => {
            write_entry(&path, false, now, &[]);
            None
        }
        // Offline: keep showing an expired photo rather than dropping it.
        Err(()) => cached.and_then(|(hit, _, img)| hit.then_some(img)),
    }
}

/// Blocking GET of one Gravatar URL.
pub fn http_get(url: &str) -> Result<Option<Vec<u8>>, ()> {
    use std::io::Read;
    match ureq::get(url).timeout(std::time::Duration::from_secs(5)).call() {
        Ok(r) => {
            let mut buf = Vec::new();
            r.into_reader().take(1 << 20).read_to_end(&mut buf).map_err(|_| ())?;
            Ok(Some(buf))
        }
        Err(ureq::Error::Status(404, _)) => Ok(None),
        Err(_) => Err(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    const DAY: u64 = 86_400;

    #[test]
    fn url_uses_the_md5_of_the_trimmed_lowercase_email() {
        // Example address from the Gravatar documentation.
        let u = gravatar_url("MyEmailAddress@example.com ", 64);
        assert_eq!(u, "https://www.gravatar.com/avatar/0bc83cb571cd1c50ba6f3e8a78ef1346?s=64&d=404");
    }

    #[test]
    fn a_hit_is_fetched_once_then_served_from_disk() {
        let d = tempfile::tempdir().unwrap();
        let calls = Cell::new(0);
        let f = |_: &str| { calls.set(calls.get() + 1); Ok(Some(vec![1, 2, 3])) };
        assert_eq!(lookup(d.path(), "a@b.c", 100, f), Some(vec![1, 2, 3]));
        assert_eq!(lookup(d.path(), "A@B.C", 200, f), Some(vec![1, 2, 3]));
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn a_404_is_cached_as_a_miss() {
        let d = tempfile::tempdir().unwrap();
        let calls = Cell::new(0);
        let f = |_: &str| { calls.set(calls.get() + 1); Ok(None) };
        assert_eq!(lookup(d.path(), "a@b.c", 100, f), None);
        assert_eq!(lookup(d.path(), "a@b.c", 100 + DAY, f), None);
        assert_eq!(calls.get(), 1);
    }

    #[test]
    fn a_network_error_is_not_cached() {
        let d = tempfile::tempdir().unwrap();
        let calls = Cell::new(0);
        let f = |_: &str| { calls.set(calls.get() + 1); Err(()) };
        assert_eq!(lookup(d.path(), "a@b.c", 100, f), None);
        assert_eq!(lookup(d.path(), "a@b.c", 101, f), None);
        assert_eq!(calls.get(), 2);
    }

    #[test]
    fn entries_expire_misses_after_7_days_and_hits_after_30() {
        let d = tempfile::tempdir().unwrap();
        let calls = Cell::new(0);
        let miss = |_: &str| { calls.set(calls.get() + 1); Ok(None) };
        let hit = |_: &str| { calls.set(calls.get() + 1); Ok(Some(vec![9])) };
        lookup(d.path(), "m@x.y", 0, miss);
        lookup(d.path(), "h@x.y", 0, hit);
        assert_eq!(calls.get(), 2);
        lookup(d.path(), "m@x.y", 8 * DAY, miss);
        lookup(d.path(), "h@x.y", 8 * DAY, hit);
        assert_eq!(calls.get(), 3, "the miss expired, the hit did not");
        lookup(d.path(), "h@x.y", 31 * DAY, hit);
        assert_eq!(calls.get(), 4);
    }

    #[test]
    fn a_stale_photo_is_still_returned_when_the_refresh_fails() {
        let d = tempfile::tempdir().unwrap();
        lookup(d.path(), "h@x.y", 0, |_| Ok(Some(vec![7])));
        assert_eq!(lookup(d.path(), "h@x.y", 40 * DAY, |_| Err(())), Some(vec![7]));
    }

    #[test]
    fn an_empty_email_never_reaches_the_network() {
        let d = tempfile::tempdir().unwrap();
        assert_eq!(lookup(d.path(), "  ", 0, |_| panic!("must not fetch")), None);
    }
}
