# Working on subnet-cloud

ICP skills are always relevant in this project. Start with them before analysis,
specification, architecture, dependency selection or implementation. Read
`docs/development.md` and the relevant app's AGENTS.md before changing that app.

<!-- ic-skills:managed:start -->
<!-- state: configured (on-demand) -->
Fetch the skills index once per session and keep each skill's name, description,
and SKILL.md URL:
https://skills.internetcomputer.org/.well-known/skills/index.json
Before writing ICP code for a task, fetch the matching skill's SKILL.md
(https://skills.internetcomputer.org/.well-known/skills/{name}/SKILL.md) and follow
it. Skills are authoritative — prefer them over general knowledge.
<!-- ic-skills:managed:end -->

## Architecture evidence before implementation

- Select skills by the actual requirement, not a preselected implementation.
  MCP/agent integration starts with service-discoverability, agent-web-identity,
  internet-identity and canister-security. Persistence/upgrades also requires
  stable-memory. Consult the index for changed names and other relevant skills.
- In the relevant spec or decision record, state: requirement; exact skill links
  and retrieval date; existing supported approach; observed gap; smallest viable
  extension; alternatives and tradeoffs; proof test; unresolved assumptions.
- Prefer existing ICP services and ordinary canister methods when they satisfy
  the requirements. Do not assume using MCP means hosting another MCP server.
- Before adding custom identity, OAuth, transport, cryptography, canisters or
  servers, prove why the supported approach is insufficient. A failed third-party
  dependency or untested assumption is not that proof. Use minimal experiments
  before integrating speculative infrastructure into the release path.
- Preserve the user's capability and provenance requirements when simplifying.
  Distinguish authenticated identity from client-supplied labels. Never silently
  replace restricted agent authority with an unrestricted human delegation.
- Make a separate review pass that challenges each new component: can it be
  removed, reused or deferred without losing a requirement? Check the evidence
  against current primary sources and adversarial tests, not just happy paths.
- Mark decisions proposed / proven / deferred. Do not freeze a spec or claim a
  platform limitation from one package's incompatibility. Reopen the decision
  when new evidence changes it; do not continue because code already exists.
- If skills are unavailable, report it and continue unaffected work. Never claim
  they were consulted. Skill checks do not authorize deployment, publishing an
  opt-in manifest, accepting external terms or sending messages.

Keep shared cloud rules in the canonical backend/SDK/UI packages. Preserve legacy
data and stable schemas, verify migrations and actual browser flows, and record
commands and limitations. Local tests do not establish production trust guarantees.

## Craft integration validation scope

User decision, 19 October 2026: for the eight `apps/*craft` integration forks,
routine upstream merges do **not** require rerunning the editor's full `xtask ci`
or thousands of unrelated tests. This scoped policy replaces inherited upstream
"full CI before every commit" instructions for our integration-update workflow.

- Review the pinned upstream delta and any available upstream CI result; record
  missing CI evidence rather than implying it passed or automatically rerunning it.
- Build each changed browser package with its pinned SDK and cloud feature; run
  the affected adapter/menu tests and check browser startup. Require an actual
  cloud save/open journey when that host's cloud hook, document lifecycle or
  serialization changed, or a concrete concern remains. Unchanged integration
  code does not require the same manual journey in all eight apps.
- Test shared cloud behavior in the canonical packages once, only when changed.
  Do not repeat the same shared suite through eight vendored copies.
- If an editor's internal behavior needs a compatibility repair, add/run the
  relevant regression and module tests. A concrete failure can justify expanding
  that scope; it does not automatically justify every editor's entire CI.
- Run a full editor suite only when explicitly requested or when a specific
  unresolved concern genuinely requires it. Explain that reason first.
- After the selected checks pass, stop. Record failures and limits; do not repair
  unrelated upstream warnings in our forks or repeatedly test unchanged code.

See `scripts/UPDATING.md` for the update procedure. This policy does not waive
focused authorization, persistence or data-integrity tests for cloud changes.

## Validation time and model-context budget

- Select the smallest relevant checks before running them, with a concrete
  reason to expand. A UI-copy edit does not trigger backend or editor suites.
- Keep verbose output in local log files. Return exit status and a short summary;
  inspect the relevant error excerpt only after a failure. Never dump binaries.
- Batch deterministic independent checks and let scripts perform repetitions.
  Avoid frequent progress polling or repeatedly reading unchanged logs/UI trees.
- Reuse results for unchanged source and artifacts. Before release, verify the
  deployed artifact and environment; do not rerun an entire successful suite.
- Use browser interaction for changed user behavior or an unresolved integration
  risk. Prefer an existing automated contract over repeating equivalent manual
  journeys; for a fleet upgrade, state explicitly which host hooks changed.
- Record commands, source/artifact identity and outcomes so later work can reuse
  evidence. Reserve broader investigation for concrete failures or new risks.
