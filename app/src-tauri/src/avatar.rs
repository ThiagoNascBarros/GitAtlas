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

/// Photo for `email`: from the disk cache while fresh, else `fetch(email)` (`Ok(None)` = no photo, `Err` = network trouble).
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
    match fetch(&email) {
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


/// A forge repository an avatar can be resolved through (only hosts with a public commit API).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Remote {
    pub host: String,
    pub owner: String,
    pub repo: String,
}

/// `git@host:o/r.git`, `https://[user@]host/o/r` and `ssh://git@host/o/r.git` for github.com and bitbucket.org.
pub fn parse_remote(url: &str) -> Option<Remote> {
    let url = url.trim();
    let (host, path) = match url.split_once("://") {
        Some((_, rest)) => {
            let (auth_host, path) = rest.split_once('/')?;
            (auth_host.rsplit('@').next()?.split(':').next()?, path)
        }
        None => url.split_once('@').map_or(url, |(_, r)| r).split_once(':')?,
    };
    if !matches!(host, "github.com" | "bitbucket.org") {
        return None;
    }
    let mut seg = path.trim_matches('/').split('/');
    let (owner, repo) = (seg.next()?, seg.next()?.trim_end_matches(".git"));
    (!owner.is_empty() && !repo.is_empty()).then(|| Remote { host: host.into(), owner: owner.into(), repo: repo.into() })
}

/// One place to try, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Cand {
    /// Download this image.
    Image(String),
    /// GET this JSON and download the image URL found at `pointer`.
    Api { url: String, pointer: &'static str },
}

fn noreply(email: &str) -> Option<String> {
    let name = |s: &str| !s.is_empty() && s.chars().all(|c| c.is_ascii_alphanumeric() || c == '-');
    let (local, domain) = email.split_once('@')?;
    match domain {
        "users.noreply.github.com" => Some(match local.split_once('+') {
            Some((id, user)) if id.chars().all(|c| c.is_ascii_digit()) && name(user) => format!("https://avatars.githubusercontent.com/u/{id}?s=64"),
            _ if name(local) => format!("https://github.com/{local}.png?size=64"),
            _ => return None,
        }),
        "users.noreply.gitlab.com" => {
            let (id, user) = local.split_once('-')?;
            (!id.is_empty() && id.chars().all(|c| c.is_ascii_digit()) && name(user))
                .then(|| format!("https://gitlab.com/uploads/-/system/user/avatar/{id}/avatar.png?width=64"))
        }
        _ => None,
    }
}

/// Where to look for `email`'s photo: the account behind a noreply address, the forge commit API
/// (needs the repo's remote and a commit that author made), then Gravatar and Libravatar.
pub fn candidates(email: &str, remote: Option<&Remote>, sha: &str) -> Vec<Cand> {
    let email = normalize(email);
    let mut v = vec![];
    if let Some(u) = noreply(&email) {
        v.push(Cand::Image(u));
    } else if let Some(r) = remote.filter(|_| (7..=64).contains(&sha.len()) && sha.chars().all(|c| c.is_ascii_hexdigit())) {
        let (o, n) = (&r.owner, &r.repo);
        v.push(match r.host.as_str() {
            "github.com" => Cand::Api { url: format!("https://api.github.com/repos/{o}/{n}/commits/{sha}"), pointer: "/author/avatar_url" },
            _ => Cand::Api { url: format!("https://api.bitbucket.org/2.0/repositories/{o}/{n}/commit/{sha}"), pointer: "/author/user/links/avatar/href" },
        });
    }
    v.push(Cand::Image(gravatar_url(&email, 64)));
    v.push(Cand::Image(format!("https://seccdn.libravatar.org/avatar/{:x}?s=64&d=404", md5::compute(&email))));
    v
}

/// Image hosts an API answer may send us to; anything else (http, IPs, look-alike domains) is ignored.
fn allowed(url: &str) -> bool {
    let Some(rest) = url.strip_prefix("https://") else { return false };
    let host = rest.split(['/', '?']).next().unwrap_or("");
    ["githubusercontent.com", "gravatar.com", "libravatar.org", "bitbucket.org", "atl-paas.net", "gitlab.com"]
        .iter()
        .any(|d| host == *d || host.strip_suffix(d).is_some_and(|p| p.ends_with('.')))
}

/// Tries each candidate: the first photo wins. A provider that failed (rate limit, offline) makes a
/// final "no photo" an `Err`, so it is not cached as a miss.
pub fn chain(cands: &[Cand], http: &dyn Fn(&str) -> Result<Option<Vec<u8>>, ()>) -> Result<Option<Vec<u8>>, ()> {
    let mut failed = false;
    let mut image = |url: &str| match http(url) {
        Ok(Some(b)) if !b.is_empty() => Some(b),
        Ok(_) => None,
        Err(()) => { failed = true; None }
    };
    for c in cands {
        let found = match c {
            Cand::Image(u) => image(u),
            Cand::Api { url, pointer } => {
                let img = match http(url) {
                    Ok(Some(body)) => serde_json::from_slice::<serde_json::Value>(&body).ok()
                        .and_then(|j| j.pointer(pointer).and_then(|v| v.as_str()).map(str::to_owned)),
                    Ok(None) => None,
                    Err(()) => None,
                };
                match img.filter(|u| allowed(u)) {
                    Some(u) if u.contains("githubusercontent.com") => image(&format!("{u}{}s=64", if u.contains('?') { "&" } else { "?" })),
                    Some(u) => image(&u),
                    None => None,
                }
            }
        };
        if found.is_some() {
            return Ok(found);
        }
    }
    if failed { Err(()) } else { Ok(None) }
}

/// The repository's `origin` (or its first remote) when it lives on a forge with a commit API.
pub fn remote_of(root: &Path) -> Option<Remote> {
    let git = |args: &[&str]| {
        let o = std::process::Command::new("git").current_dir(root).args(args).output().ok()?;
        o.status.success().then(|| String::from_utf8_lossy(&o.stdout).trim().to_owned()).filter(|s| !s.is_empty())
    };
    let name = git(&["remote", "get-url", "origin"]).or_else(|| git(&["remote"]).and_then(|r| git(&["remote", "get-url", r.lines().next()?])))?;
    parse_remote(&name)
}

/// Newest commit by `email` that already exists on a remote: the forge only knows pushed commits,
/// and the commit on top of the graph is often still local.
pub fn pushed_sha(root: &Path, email: &str) -> Option<String> {
    let email = normalize(email);
    let out = std::process::Command::new("git").current_dir(root)
        .args(["log", "--remotes", "-n", "50", "-F", "-i", "--format=%H %ae", "--author", &email])
        .output().ok().filter(|o| o.status.success())?;
    String::from_utf8_lossy(&out.stdout).lines()
        .filter_map(|l| l.split_once(' '))
        .find(|(_, ae)| ae.eq_ignore_ascii_case(&email))
        .map(|(sha, _)| sha.to_owned())
}

/// Blocking GET of one URL (`Ok(None)` on 404).
pub fn http_get(url: &str) -> Result<Option<Vec<u8>>, ()> {
    use std::io::Read;
    match ureq::get(url).set("User-Agent", "GitAtlas").timeout(std::time::Duration::from_secs(5)).call() {
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

#[cfg(test)]
mod provider_tests {
    use super::*;
    use std::cell::RefCell;

    fn urls(c: &[Cand]) -> Vec<String> {
        c.iter().map(|c| match c { Cand::Image(u) => u.clone(), Cand::Api { url, .. } => url.clone() }).collect()
    }
    fn gh() -> Remote { Remote { host: "github.com".into(), owner: "octo".into(), repo: "hello".into() } }

    #[test]
    fn remotes_parse_from_https_scp_and_ssh_forms() {
        for u in ["git@github.com:octo/hello.git", "https://github.com/octo/hello", "https://tok@github.com/octo/hello.git", "ssh://git@github.com/octo/hello.git"] {
            assert_eq!(parse_remote(u), Some(gh()), "{u}");
        }
        assert_eq!(parse_remote("https://bitbucket.org/ws/r.git").map(|r| r.host), Some("bitbucket.org".into()));
        assert_eq!(parse_remote("https://example.com/a/b.git"), None);
        assert_eq!(parse_remote("/local/path"), None);
    }

    #[test]
    fn github_noreply_emails_map_straight_to_the_account_avatar() {
        let a = candidates("12345+octo@users.noreply.github.com", None, "");
        assert_eq!(urls(&a)[0], "https://avatars.githubusercontent.com/u/12345?s=64");
        let b = candidates("octo@users.noreply.github.com", None, "");
        assert_eq!(urls(&b)[0], "https://github.com/octo.png?size=64");
    }

    #[test]
    fn gitlab_noreply_emails_map_to_the_account_avatar() {
        let a = candidates("1234-someone@users.noreply.gitlab.com", None, "");
        assert_eq!(urls(&a)[0], "https://gitlab.com/uploads/-/system/user/avatar/1234/avatar.png?width=64");
    }

    #[test]
    fn a_real_email_asks_the_forge_commit_api_then_gravatar_then_libravatar() {
        let sha = "a".repeat(40);
        let u = urls(&candidates("Me@Example.com", Some(&gh()), &sha));
        assert_eq!(u[0], format!("https://api.github.com/repos/octo/hello/commits/{sha}"));
        assert!(u[1].starts_with("https://www.gravatar.com/avatar/"));
        assert!(u[2].starts_with("https://seccdn.libravatar.org/avatar/"));
        let bb = Remote { host: "bitbucket.org".into(), owner: "ws".into(), repo: "r".into() };
        assert_eq!(urls(&candidates("me@example.com", Some(&bb), &sha))[0], format!("https://api.bitbucket.org/2.0/repositories/ws/r/commit/{sha}"));
    }

    #[test]
    fn noreply_and_non_hex_shas_never_hit_the_commit_api() {
        let sha = "b".repeat(40);
        assert!(urls(&candidates("octo@users.noreply.github.com", Some(&gh()), &sha)).iter().all(|u| !u.contains("api.github.com")));
        assert!(urls(&candidates("me@example.com", Some(&gh()), "../../x")).iter().all(|u| !u.contains("api.github.com")));
    }

    fn run(c: &[Cand], answers: &[(&str, Result<Option<Vec<u8>>, ()>)]) -> (Result<Option<Vec<u8>>, ()>, Vec<String>) {
        let seen = RefCell::new(vec![]);
        let r = chain(c, &|u: &str| {
            seen.borrow_mut().push(u.to_owned());
            answers.iter().find(|(k, _)| u.starts_with(k)).map_or(Ok(None), |(_, v)| v.clone())
        });
        (r, seen.into_inner())
    }

    #[test]
    fn the_first_hit_wins_and_stops_the_chain() {
        let c = [Cand::Image("https://a/1".into()), Cand::Image("https://b/2".into())];
        let (r, seen) = run(&c, &[("https://a/", Ok(Some(vec![1])))]);
        assert_eq!(r, Ok(Some(vec![1])));
        assert_eq!(seen, ["https://a/1"]);
    }

    #[test]
    fn a_failing_provider_is_skipped_but_blocks_a_final_miss() {
        let c = [Cand::Image("https://a/1".into()), Cand::Image("https://b/2".into())];
        assert_eq!(run(&c, &[("https://a/", Err(()))]).0, Err(()), "rate limit must not be cached as 'no photo'");
        assert_eq!(run(&c, &[("https://a/", Err(())), ("https://b/", Ok(Some(vec![2])))]).0, Ok(Some(vec![2])));
        assert_eq!(run(&c, &[]).0, Ok(None));
    }

    #[test]
    fn an_api_answer_points_at_the_image_to_download() {
        let c = [Cand::Api { url: "https://api.github.com/x".into(), pointer: "/author/avatar_url" }];
        let json = br#"{"author":{"avatar_url":"https://avatars.githubusercontent.com/u/9?v=4"}}"#.to_vec();
        let (r, seen) = run(&c, &[("https://api.github.com/", Ok(Some(json))), ("https://avatars.githubusercontent.com/", Ok(Some(vec![7])))]);
        assert_eq!(r, Ok(Some(vec![7])));
        assert_eq!(seen[1], "https://avatars.githubusercontent.com/u/9?v=4&s=64");
    }

    #[test]
    fn an_api_answer_cannot_redirect_us_to_an_unknown_host() {
        let c = [Cand::Api { url: "https://api.github.com/x".into(), pointer: "/author/avatar_url" }];
        for bad in ["http://avatars.githubusercontent.com/u/1", "https://evil.example/a.png", "https://127.0.0.1/a", "https://githubusercontent.com.evil.io/a"] {
            let json = format!(r#"{{"author":{{"avatar_url":"{bad}"}}}}"#).into_bytes();
            let (r, seen) = run(&c, &[("https://api.github.com/", Ok(Some(json)))]);
            assert_eq!(r, Ok(None), "{bad}");
            assert_eq!(seen.len(), 1, "{bad} must not be requested");
        }
    }
}

#[cfg(test)]
mod pushed_tests {
    use super::*;

    fn git(d: &Path, args: &[&str]) -> String {
        let o = std::process::Command::new("git").current_dir(d).args(args)
            .env("GIT_AUTHOR_NAME", "A").env("GIT_COMMITTER_NAME", "A").env("GIT_COMMITTER_EMAIL", "c@x.y")
            .output().unwrap();
        assert!(o.status.success(), "{args:?}: {}", String::from_utf8_lossy(&o.stderr));
        String::from_utf8_lossy(&o.stdout).trim().to_owned()
    }

    fn commit_as(d: &Path, email: &str, msg: &str) -> String {
        let o = std::process::Command::new("git").current_dir(d)
            .args(["-c", &format!("user.email={email}"), "-c", "user.name=N", "commit", "--allow-empty", "-q", "-m", msg])
            .output().unwrap();
        assert!(o.status.success());
        git(d, &["rev-parse", "HEAD"])
    }

    #[test]
    fn picks_the_authors_newest_commit_that_exists_on_a_remote() {
        let d = tempfile::tempdir().unwrap();
        git(d.path(), &["init", "-q", "-b", "main"]);
        let pushed = commit_as(d.path(), "me@x.y", "one");
        git(d.path(), &["update-ref", "refs/remotes/origin/main", &pushed]);
        commit_as(d.path(), "me@x.y", "local only");
        commit_as(d.path(), "other@x.y", "someone else, local");
        assert_eq!(pushed_sha(d.path(), "ME@x.y"), Some(pushed));
        assert_eq!(pushed_sha(d.path(), "other@x.y"), None);
        assert_eq!(pushed_sha(d.path(), "nobody@x.y"), None);
    }

    #[test]
    fn an_email_with_regex_characters_matches_literally() {
        let d = tempfile::tempdir().unwrap();
        git(d.path(), &["init", "-q", "-b", "main"]);
        let sha = commit_as(d.path(), "a.b+c@x.y", "one");
        git(d.path(), &["update-ref", "refs/remotes/origin/main", &sha]);
        assert_eq!(pushed_sha(d.path(), "a.b+c@x.y"), Some(sha));
        assert_eq!(pushed_sha(d.path(), "a.*@x.y"), None);
    }
}
