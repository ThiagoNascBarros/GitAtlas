# Feature Specification: Author avatars in the commit graph

**Status**: In progress
**Decision**: Gravatar always on (owner choice, 2026-10-25). Sends the MD5 of each author's e-mail to gravatar.com. Initials are the fallback.

## Acceptance Scenarios
1. **Given** a commit row, **Then** the graph node is a circular avatar of its author with a ring in the lane colour.
2. **Given** an author without a Gravatar (404) or no network, **Then** the node shows initials on a colour derived from the e-mail, and the miss is cached 7 days.
3. **Given** a found photo, **Then** it is fetched by the Rust side only (webview CSP unchanged), cached on disk 30 days, and fetched once per author.
4. **Given** 1000 visible authors, **Then** at most 4 requests are in flight and drawing never waits for them.
