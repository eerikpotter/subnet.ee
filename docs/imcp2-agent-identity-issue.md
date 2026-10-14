# Support application verifiable restricted agent identities

Published 2026-10-14: https://github.com/dfinity/imcp2/issues/208.
The text below is the preparation copy; the linked issue is authoritative.

Prepared for `dfinity/imcp2`; publication authorized by the user on 14 October 2026.

## Problem

We are building an ICP-native shared drive at subnet.ee and want to use the existing
ICP MCP connector with ordinary canister APIs. A user should eventually be able to
give an AI less authority than their own account while retaining accountable
attribution: for example, Bob can edit Alice's folder, but Bob's AI can only read
and propose changes there. Alice need not separately approve Bob's use of AI.

This is a feature request for a restricted connection mode, not a claim that the
documented full-user-authority mode is broken.

## Observed behavior

The identity path in imcp2 commit
`355d8deb181c5cf0486de5c3ad8be93c5f2dfa76` obtains an II delegation for the user's
app account. We exercised the corresponding II methods with backend release
`release-2026-10-13`, PocketIC 16.0.0, ic-agent 0.49.2 and ic-cdk 0.20.3:

1. Register a disposable II identity and configure a trusted MCP origin.
2. Obtain a normal app delegation via `prepare_delegation` / `get_delegation`.
3. Obtain a registration delegation, call `mcp_register_v2`, then obtain the same
   app's delegation via `mcp_prepare_delegation` / `mcp_get_delegation`.
4. Use both identities over signed HTTP ingress to a probe returning
   `msg_caller()`, `msg_caller_info_signer()` and `msg_caller_info_data()`.

Both full-access paths produced the same caller, no caller-info signer and empty
caller-info data. The full MCP delegation could invoke an ordinary endpoint
authorized for that user. Query-only delegations were enforced by the replica,
but they do not express per-folder or per-operation application permissions.
Some applications also use updates for consensus-backed metadata reads.

This experiment verifies II-issued delegations and application calls, not the
hosted MCP transport or interactive browser consent. It does not establish that
no other supported ICP mechanism can meet the requirement.

## Desired capability

A connection mode in which the application can verify an app-scoped connection
identity and its sponsoring user, then enforce its own narrower grants on every
request. The connector must not also retain an unrestricted credential for that
same app that bypasses those grants. Missing or stripped context must not be
interpreted as an ordinary user session.

A separate principal registered by the user worked in our control experiment:
the canister attributed its calls to that connection/user, denied edits and grant
administration, and denied reads immediately after app revocation. The open
question is how to provision that boundary through standard ICP MCP / II.

Could this use an existing II flow, a distinct app-scoped principal, or trusted
caller-info with enforced binding? Which parts would need changes in imcp2,
Internet Identity or the replica protocol? A documented supported recipe would
also help if the capability already exists.

## Related work and alpha fallback

- [Scoped delegation discussion](https://forum.dfinity.org/t/adding-parameters-to-delegated-identity/33071)
- [Post-revocation window](https://github.com/dfinity/imcp2/issues/121)
- [Inspected identity code](https://github.com/dfinity/imcp2/blob/355d8deb181c5cf0486de5c3ad8be93c5f2dfa76/crates/imcp2-core/src/identities.rs)

For the alpha we will use the current full-user-authority mode and disclose that
limitation. We are seeking a small reusable extension rather than maintaining a
second MCP/authentication stack in each application.
