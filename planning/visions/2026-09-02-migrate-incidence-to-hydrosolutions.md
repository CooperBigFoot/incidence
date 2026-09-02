# Migrate Incidence to the hydrosolutions GitHub organization

Incidence currently lives at `CooperBigFoot/incidence`, while its downstream Taqsim consumer already lives in the `hydrosolutions` organization. Move Incidence into the same organization and replace every tracked Taqsim reference to the old GitHub namespace.

## Intended outcome

The canonical Incidence repository is `https://github.com/hydrosolutions/incidence`. The move retains the existing project rather than creating a disconnected copy. Repository history and GitHub-hosted project records remain intact, and Taqsim no longer relies on the former owner URL or GitHub's redirect behavior.

## Settled scope

- Transfer `CooperBigFoot/incidence` to `hydrosolutions/incidence` using GitHub's repository ownership transfer.
- Keep the repository name `incidence`.
- Keep it public.
- Preserve its Git history, issues, pull requests, releases, and repository settings through the transfer.
- Change this local Incidence checkout's `origin` to `https://github.com/hydrosolutions/incidence.git`.
- In the `hydrosolutions/taqsim` repository, replace all tracked canonical references to `CooperBigFoot/incidence` with `hydrosolutions/incidence`. Known references include:
  - the human-facing Incidence link in `README.md` on Taqsim's remote `main` branch;
  - the Git dependency source in `pyproject.toml`;
  - the resolved Git source in `uv.lock`.
- Preserve the pinned Incidence revision unless dependency resolution proves a change is required. Ownership transfer does not itself change the commit identity.
- Preserve unrelated work in existing local checkouts.

This work does not rename Incidence, change visibility, redesign either project, or change dependency behavior beyond adopting the canonical repository URL.

## Repository facts observed during discovery

- Incidence's current remote is `https://github.com/CooperBigFoot/incidence.git`.
- GitHub reports `CooperBigFoot/incidence` as public with `main` as its default branch.
- The authenticated GitHub account is an active administrator of `hydrosolutions` and has administrative permission on `CooperBigFoot/incidence`.
- `hydrosolutions/incidence` did not exist when checked, so the intended destination name was available.
- Taqsim's remote is already `https://github.com/hydrosolutions/taqsim.git`.
- The existing Incidence checkout was 37 commits behind `origin/main` during discovery.
- The existing Taqsim checkout was 28 commits behind `origin/main` and contained unrelated modifications to `CONTEXT.md` plus an untracked ADR. Those changes must not be overwritten or included accidentally.
- Taqsim's local README was stale and lacked the link, but `origin/main:README.md` contained `https://github.com/CooperBigFoot/incidence`. Work must be based on the current remote branch rather than the stale local file.

## Observable evidence of completion

- `https://github.com/hydrosolutions/incidence` resolves to the transferred public repository and identifies `hydrosolutions/incidence` as its canonical owner/name.
- Existing Incidence history and GitHub project records remain available after transfer.
- The Incidence implementation checkout fetches from and pushes to the new canonical remote.
- A search of tracked files in the updated Taqsim revision finds no `CooperBigFoot/incidence` references.
- Taqsim's README links to `https://github.com/hydrosolutions/incidence`.
- Taqsim's declared and locked Incidence Git sources use the new organization URL while retaining a valid pinned revision.
- Taqsim dependency resolution and its relevant repository checks succeed after the URL change.
- The migration and Taqsim reference update are committed and published through the normal review path without absorbing unrelated local changes.

## Risks and execution constraints

Repository transfer is an externally visible administrative operation. Verify destination availability and permissions immediately before executing it, then verify the canonical repository and access immediately afterward. GitHub normally redirects the previous URL, but the new namespace must be used directly everywhere under project control.

The two observed local checkouts are stale, and Taqsim contains unrelated work. Use clean, current branches or isolated worktrees for publication. Do not reset, clean, stash, amend, or otherwise disturb the pre-existing Taqsim working tree merely to perform this migration.
