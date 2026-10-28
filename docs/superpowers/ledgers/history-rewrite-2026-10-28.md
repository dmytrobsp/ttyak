# History rewrite, 2026-10-28

On the maintainer's word (2026-10-28), `master` was rewritten from `d062ce6` (2026-10-27) on to take the name of the
private claim tool out of the tracked plans, ledgers and design; it is named only through `AGENTS.local.md` (in
these files: `<tool>`). Nothing else changed: the same files, authors, dates and messages. Commits before `d062ce6`
and every release tag keep their hashes.

The ledgers, the progress page and the history name ttyak commits of that range by their old hashes. This table maps
each old hash to the new one.

| Old | New | Commit |
|---|---|---|
| d062ce6 | 65c0ad1 | docs: design for test machines, claims and throwaway instances (#30) |
| 51d0615 | 7a0d26f | docs: plan and ledger for box claims (#30, parts 1-2) |
| 09a2c03 | a730ec5 | docs: box-claims ledger — Task 1 complete |
| c96993f | 9175ed1 | docs: box-claims ledger — Tasks 2 and 3 complete |
| 575d7eb | 88524e6 | docs: box-claims ledger — Task 4 complete |
| a6e75ec | c34ae75 | docs: box-claims ledger — Task 5 complete |
| aba8fa4 | c2b8f7a | docs: box-claims ledger — Task 6 in progress, a refused step |
| fed8b50 | 35008b9 | docs: box-claims ledger — Task 6 complete, live on both boxes |
| e5318bb | ed1a16f | docs: shared test boxes are used only under a claim |
| 16501c6 | b815875 | docs: test-machine recipes run through the claim tool; claim waits are not capped |
| 554a26a | be0fea3 | docs: box-claims ledger — Task 7 complete |
| d8d6be8 | 18cd265 | docs: box claims done — ledger closed, progress page rewritten (#30) |
| e409c53 | f509c43 | docs: box-claims ledger — the deferred findings fixed, not filed |
| 8295fba | 75fcc88 | docs: review findings in scope are fixed, not filed |
| aaace61 | 4d89bde | docs: plan and ledger for account claims (#30, part 4) |
| b6965ae | 146d28a | tools: accounts ledger and per-account claim rules (#30) |
| 5ef7ae7 | 2c729f5 | tools: accounts claims refuse symlinks and loose folders, tidy write paths (#30) |
| 6f753d4 | 9f3a8ed | tools: accounts claims read state without following symlinks, validate queue (#30) |
| 7c60978 | 2f51807 | docs: account-claims ledger — Task 1 complete |
| 508cbb8 | fe0fbb0 | tools: accounts claim commands (list, status, claim, renew, release), claim-record validation (#30) |
| a727ab9 | 0cf81d6 | tools: accounts claims survive FIFOs, safe display, resume line with dash values (#30) |
| ea811fb | 175e46e | tools: accounts import, install and release cleanup of session copies (#30) |
| 3a881f4 | 8d5ffa1 | tools: accounts session moves check folder identity, clean up before saving (#30) |
| 8e05561 | 58a53f1 | docs: account-claims ledger — Tasks 2 and 3 complete |
| 0152e72 | 4c5065a | docs: live checks claim test accounts and install sessions with accounts.py (#30) |
| 2fde28c | d5f6c95 | docs: live-testing and setup take sessions from the ledger, release at the end (#30) |
| 3fb7771 | a145f2c | docs: rewrap two lines in live-testing.md (#30) |
| 7e724f0 | a5213cb | docs: rewrap a long line in live-testing.md (#30) |
| 113ac89 | 6c1d792 | docs: setup.md shows how the account ledger is made and filled (#30) |
| 60aa44d | 0bef3ac | docs: absolute accounts.py path in setup.md, reflow a line in live-testing.md (#30) |
| 19d44d0 | 8178263 | docs: rewrap a line in setup.md (#30) |
| 2a15055 | 490a1aa | docs: account-claims ledger — Task 4 complete, live check recorded |
| 7de7503 | 6a1d27f | accounts: SIGTERM leaves the queue like Ctrl+C, missing-folder hint, label Linux-only install (#30) |
| 59deb98 | 3fc881e | docs: claim lease, release order, boxes and worktrees in the account-claims docs (#30) |
| 66a210d | e98d270 | docs: rewrap a line in local-template.md (#30) |
| 99f6ec9 | ccb395f | docs: rewrap a line in local-template.md again (#30) |
| 743b538 | 89f646e | docs: account-claims plan gains Task 5 (recipes off the real config folder); ledger |
| d9f8690 | 08030aa | launch.sh: run ttyak in a fake home from TTYAK_FAKE_HOME, refuse without it (#30) |
| 629cc35 | 9e768ea | docs: agent recipes run test profiles in fake homes, never the real config folder (#30) |
| ab53003 | 0965f15 | launch.sh: refuse a relative TTYAK_FAKE_HOME, lines within 120 columns (#30) |
| 1d08267 | 81f3d43 | docs: account claims done — ledger closed, progress page rewritten (#30) |
| b8661ed | afabfb7 | docs: plan and ledger for Linux instances; the Hyprland spike in the design (#30) |
| 283b439 | dbab59d | docs: linux-instances ledger — Task 1 complete |
| d8030f3 | fc7f082 | docs: linux-instances ledger — Tasks 2 and 3 complete |
| 2aa0562 | b8c4bd3 | docs: linux-instances ledger — Task 4 (GNOME) complete |
| 6c424f2 | 7f67ef4 | docs: linux-instances ledger — ttyak live in a sway instance |
| 1aa7622 | cf41c98 | docs: live-testing — a Linux instance is the default place for Linux live checks |
| 4839ba4 | 90d64c9 | docs: live-testing — instance section: say what is not tried, sound note, wording |
| 0e111f0 | c8e9818 | docs: linux-instances ledger — Task 5 docs reviewed |
| 3bddb41 | e9d9325 | docs: live-testing — instance recipe: renew + bounded ttyak, claim id, fake-home cleanup, Slot wording |
| 8b3b5c2 | a491ef3 | docs: Linux instances done — ledger closed, progress page rewritten (#30) |
| 9534131 | 18aa24e | docs: test-machines design — VM clones spike: Hyprland VM and Windows evaluation base work in user-mode libvirt (#30) |
| 0ee9cab | 3299b13 | docs: test-machines design — VM clones: the maintainer's Defender call, my calls, answers for the builder (#30) |
| a6303cd | 00e29b2 | docs: VM clones plan, its ledger, progress page (#30) |
| 85d78fc | 3f4a9b8 | docs: VM clones plan — rewrap a long line |
| 2ae376d | a8252fe | docs: vm-clones ledger — plan started, Task 1 dispatched |
| bf5da4b | a33c333 | docs: vm-clones ledger — Task 1 reviewed, fix round 1 |
| 8fa39ab | f3a19fd | docs: vm-clones ledger — Task 1 complete, Task 2 dispatched |
| 02d5a60 | 701ecb7 | docs: vm-clones ledger — Task 2 reviewed, fix round 1, a ruling |
| fd2da46 | f5149bd | docs: vm-clones ledger — Task 2 complete, Task 3 dispatched |
| 44a1c06 | 52f6173 | docs: vm-clones ledger — Task 3 reviewed, fix round 1 |
| 1c35f38 | ebc247d | docs: vm-clones ledger — Task 3 fix round 1 re-reviewed, round 2 |
| f2465bf | c998156 | docs: vm-clones ledger — Task 3 complete, Task 4 dispatched |
| 1c9feff | e48b8df | docs: vm-clones ledger — Task 4 reviewed, fix round 1 |
| bef7ac1 | e0d7bbd | docs: vm-clones ledger — Task 4 fix round 1 re-reviewed, round 2 |
| 061b4f2 | b045dc2 | docs: vm-clones ledger — Task 4 complete, Task 5 dispatched |
| eed8352 | 23db1f2 | docs: vm-clones ledger — Task 5 spike, build, review, two rulings |
| 7e40ccf | 68fee0e | docs: vm-clones ledger — Task 5 fix round 1 re-reviewed, round 2 |
| 006e5b8 | acce5e9 | docs: vm-clones ledger — Task 5 complete, Task 6 dispatched |
| e8a5d5e | 068eecb | docs: vm-clones ledger — Task 6 built, reviewed, three rulings |
| 7cec18f | e0381bb | docs: vm-clones ledger — Task 6 complete, Task 7 dispatched |
| f6cfb0f | 14d06d4 | docs: live-testing — ttyak in the Hyprland VM and in Windows clones (#30) |
| 46740a8 | 853725f | docs: vm-clones ledger — Task 7 live checks (Hyprland VM, Windows clone), #63 filed |
| 7d9ebef | 3e64d0e | docs: live-testing — VM clones, live results and public wording (#30) |
| 1dbe9e9 | 2c5a321 | docs: live-testing — wrap a long line (#30) |
| 8bdcc3b | 9d9971a | docs: vm-clones ledger — Task 7 complete, final review |
| b5b9911 | eb46aae | docs: vm-clones ledger — final review, a ruling, fix wave |
| de31f9b | 8b8e134 | docs: live-testing — Windows recipe sets and releases the account, a file mount is refused (#30) |
| 73a8f8e | 6a3d1a2 | docs: VM clones done — ledger closed, progress page rewritten (#30) |
| 0fe305c | 81ef3ed | docs: test-machines design — macOS VM spike, my calls for the plan (#30) |
