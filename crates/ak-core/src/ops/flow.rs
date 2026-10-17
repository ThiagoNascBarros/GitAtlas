// SPDX-License-Identifier: Apache-2.0
//! Branch naming and Git Flow base rules (pure functions, no repository access).
//!
//! The name is `prefix + compose(template, ticket, title)`. The template may use `{ticket}`
//! and `{title}`; whitespace becomes `-` and characters Git refuses are dropped, so
//! `"{ticket} {title}"` with `ACT-1010` and `Login com SSO` gives `ACT-1010-Login-com-SSO`.

use serde::Serialize;

/// Kind chosen in the "New branch" dialog. `Free` and `Fix` have no Git Flow base rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BranchKind {
    Free,
    Feature,
    Fix,
    Release,
    Hotfix,
}

impl BranchKind {
    /// Parses the lowercase name used by the UI (`"free"`, `"feature"`, ...).
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "free" => Self::Free,
            "feature" => Self::Feature,
            "fix" => Self::Fix,
            "release" => Self::Release,
            "hotfix" => Self::Hotfix,
            _ => return None,
        })
    }

    /// Prefix added in front of the composed name (empty for `Free`).
    pub fn prefix(self) -> &'static str {
        match self {
            Self::Free => "",
            Self::Feature => "feature/",
            Self::Fix => "fix/",
            Self::Release => "release/",
            Self::Hotfix => "hotfix/",
        }
    }
}

/// Result of planning a branch: the full name and the commit it starts from (`HEAD` for none).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BranchPlan {
    pub name: String,
    pub base: String,
}

/// Builds the full branch name and its base. Git Flow rules: feature and release start from
/// `develop`, hotfix from `main`; free and fix branches start from `HEAD`.
pub fn plan(
    kind: BranchKind,
    template: &str,
    ticket: &str,
    title: &str,
    main: &str,
    develop: &str,
) -> BranchPlan {
    let base = match kind {
        BranchKind::Feature | BranchKind::Release => develop,
        BranchKind::Hotfix => main,
        BranchKind::Free | BranchKind::Fix => "HEAD",
    };
    // An empty composed part means there is no name yet: return "" rather than a bare prefix.
    let tail = compose(template, ticket, title);
    let name = if tail.is_empty() {
        String::new()
    } else {
        format!("{}{tail}", kind.prefix())
    };
    BranchPlan {
        name,
        base: base.to_owned(),
    }
}

/// Fills `{ticket}` and `{title}` in `template` and sanitizes the result into a ref-safe segment.
/// May return an empty string; the engine then rejects the name.
pub fn compose(template: &str, ticket: &str, title: &str) -> String {
    let filled = template
        .replace("{ticket}", ticket.trim())
        .replace("{title}", title.trim());
    sanitize(&filled)
}

fn sanitize(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for c in raw.chars() {
        match c {
            c if c.is_whitespace() => out.push('-'),
            // Characters Git forbids in refs, plus braces so no "@{" can survive.
            '~' | '^' | ':' | '?' | '*' | '[' | '\\' | '{' | '}' | '"' | '<' | '>' | '|' => {}
            c if c.is_control() => {}
            c => out.push(c),
        }
    }
    // Collapse separator runs and strip the edges Git rejects ("..", leading "-" or ".", trailing "/").
    let mut collapsed = String::with_capacity(out.len());
    for c in out.chars() {
        let repeats = matches!(c, '-' | '.' | '/') && collapsed.ends_with(c);
        if !repeats {
            collapsed.push(c);
        }
    }
    collapsed
        .trim_matches(|c| matches!(c, '-' | '.' | '/'))
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compose_replaces_spaces_and_keeps_ticket() {
        assert_eq!(
            compose("{ticket} {title}", "ACT-1010", "Login com SSO"),
            "ACT-1010-Login-com-SSO"
        );
    }

    #[test]
    fn compose_without_ticket_has_no_leading_separator() {
        assert_eq!(compose("{ticket} {title}", "", "  Ajuste  "), "Ajuste");
    }

    #[test]
    fn compose_drops_characters_git_refuses() {
        assert_eq!(
            compose("{title}", "", "a~b^c:d?e*f[g\\h@{i}..j//k"),
            "abcdefgh@i.j/k"
        );
        assert_eq!(compose("{title}", "", "x..y"), "x.y");
        assert_eq!(compose("{title}", "", "x  y"), "x-y");
    }

    #[test]
    fn compose_trims_edges_and_handles_empty() {
        assert_eq!(compose("{ticket} {title}", "", ""), "");
        assert_eq!(compose("-{title}-", "", "a"), "a");
    }

    #[test]
    fn unknown_placeholders_are_removed_not_kept() {
        assert_eq!(compose("{ticket}-{foo}", "T1", ""), "T1-foo");
    }

    #[test]
    fn free_branch_has_no_prefix_and_starts_at_head() {
        let p = plan(
            BranchKind::Free,
            "{ticket} {title}",
            "ACT-1",
            "Teste",
            "main",
            "develop",
        );
        assert_eq!(
            p,
            BranchPlan {
                name: "ACT-1-Teste".into(),
                base: "HEAD".into()
            }
        );
    }

    #[test]
    fn git_flow_kinds_use_prefix_and_base_rules() {
        let f = plan(
            BranchKind::Feature,
            "{ticket} {title}",
            "ACT-1010",
            "Login",
            "main",
            "develop",
        );
        assert_eq!(
            f,
            BranchPlan {
                name: "feature/ACT-1010-Login".into(),
                base: "develop".into()
            }
        );
        let r = plan(BranchKind::Release, "{title}", "", "1.2.0", "prod", "dev");
        assert_eq!(
            r,
            BranchPlan {
                name: "release/1.2.0".into(),
                base: "dev".into()
            }
        );
        let h = plan(BranchKind::Hotfix, "{title}", "", "crash", "prod", "dev");
        assert_eq!(
            h,
            BranchPlan {
                name: "hotfix/crash".into(),
                base: "prod".into()
            }
        );
        let x = plan(
            BranchKind::Fix,
            "{ticket} {title}",
            "ACT-2",
            "bug",
            "main",
            "develop",
        );
        assert_eq!(
            x,
            BranchPlan {
                name: "fix/ACT-2-bug".into(),
                base: "HEAD".into()
            }
        );
    }

    #[test]
    fn empty_name_stays_empty_even_with_a_prefix() {
        let p = plan(
            BranchKind::Feature,
            "{ticket} {title}",
            "",
            "  ",
            "main",
            "develop",
        );
        assert_eq!(
            p,
            BranchPlan {
                name: String::new(),
                base: "develop".into()
            }
        );
    }

    #[test]
    fn parse_accepts_only_known_kinds() {
        assert_eq!(BranchKind::parse("hotfix"), Some(BranchKind::Hotfix));
        assert_eq!(BranchKind::parse("Feature"), None);
        assert_eq!(BranchKind::parse("main"), None);
    }
}
