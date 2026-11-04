# CLAUDE.md — Charlotte

Living knowledge file for this project. Update it whenever something new is learned
about the project (purpose, stack, structure, commands, conventions, decisions).

## Project
- **Name:** Charlotte
- **Purpose:** _TBD (to be provided by the owner)_
- **Status:** Empty repository, `main` branch, no commits yet (as of 2026-11-04).

## Environment
- Code lives on a Debian machine, accessed over SSH from the owner's Mac.
  - Host `192.168.2.32`, user `jeansamist`, SSH alias `debian` (key `~/.ssh/id_ed25519_debian` on the Mac).
  - Project path: `/home/jeansamist/Charlotte`.
- Debian testing (forky/sid), kernel 7.1.x, 12 cores, 15 GB RAM.
- **Root disk is ~94% full (≈3 GB free)** — watch out with large installs, Docker images, `node_modules`.
- Tooling seen in home dir (not yet confirmed for this project): bun, cargo, AdonisJS v6 history.

## Sync workflow (Mac <-> GitHub <-> Debian)
- GitHub remote: `git@github-jeansamist:jeansamist/Charlotte.git` (branch `main`), same on both machines.
- Local copy (Mac): `/Users/jeansamist/lab/me/Charlotte`. Remote copy (Debian): `/home/jeansamist/Charlotte`.
- **Change made on the Mac** -> commit + push from the Mac -> `ssh debian 'cd ~/Charlotte && git pull'`.
- **Change made on Debian** -> commit + push on Debian -> `git pull` on the Mac.
- Always pull on the target side before editing, so the two copies never drift.

## Stack / Architecture
_TBD_

## Commands
_TBD (install, dev, build, test, lint)_

## Conventions
_TBD_

## Documentation to write (pending owner input)
- [ ] `README.md`
- [ ] Developer guide
