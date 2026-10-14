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
