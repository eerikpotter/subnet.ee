# Scoped agent access and attribution through ICP MCP

Published in Developers on 2026-10-14:
https://forum.dfinity.org/t/scoped-agent-access-and-attribution-through-icp-mcp/76096

This is the preparation copy. The published post links to
https://github.com/dfinity/imcp2/issues/208 and is authoritative.

We are building an ICP-native shared drive for subnet.ee. People can share files
and folders, work in web editors, and connect an AI through MCP. We would prefer
to use the existing ICP MCP connector with ordinary canister methods, following
the [service-discoverability skill](https://skills.internetcomputer.org/skills/service-discoverability/),
instead of implementing another MCP server and authentication stack.

The missing piece for our desired permission model is a trusted distinction
between a user session and an agent connection acting for that user.

For example, Alice gives Bob Edit access to a folder. Bob should be able to give
his AI only Read and Propose access there, with no extra approval from Alice.
The canister would intersect Bob's current rights with his explicit grant to that
AI connection. It would also record which connection acted for Bob, and let Bob
revoke that connection independently. We mean authenticated credential provenance,
not proof that a language model generated the action.

## Current understanding

In [imcp2 at 355d8de](https://github.com/dfinity/imcp2/blob/355d8deb181c5cf0486de5c3ad8be93c5f2dfa76/crates/imcp2-core/src/identities.rs),
per-app requests use an Internet Identity delegation for the user's app account.
[II's MCP implementation at 2f175eb](https://github.com/dfinity/internet-identity/blob/2f175eb56a339458618e218f8862fa121a621502/src/internet_identity/src/mcp.rs)
supports a query-only restriction and limits new app delegations to five minutes.
That is useful, but it does not express permissions on particular folders or
distinguish Propose, Create, Edit and Delete inside one canister. Protocol query
access also differs from application Read access: some applications use updates
for consensus-backed metadata reads.

In a local experiment using II backend release `release-2026-10-13`, PocketIC
16.0.0 and ic-agent 0.49.2, real II-issued app and MCP delegations produced
identical `msg_caller()` values, absent `msg_caller_info_signer()` and empty
`msg_caller_info_data()`. The full MCP delegation could call an endpoint
authorized for the user. The replica rejected an update under a query-only
delegation. Revoking MCP in II blocked new issuance while an already issued
delegation remained usable before expiry. Calls used signed HTTP ingress and
verified canister signatures; browser consent clicks and the MCP transport were
not exercised in this experiment.

As a control, the same probe gave a distinct signing principal read-only access
linked to the user. It rejected edits, grant administration and reads after app
revocation. That proves the application boundary, not existing imcp2 support for
issuing such a credential.

The [2024 discussion about scoped delegations](https://forum.dfinity.org/t/adding-parameters-to-delegated-identity/33071)
recommended a separate principal with application-enforced restrictions. That
looks like a suitable direction. IC CDK now also exposes `msg_caller_info_data`
and `msg_caller_info_signer`, but their presence alone does not establish that
the MCP flow supplies a trustworthy connection identifier.

## Questions

1. Is there a supported ICP MCP / II flow that provides an application with a
   distinct agent-connection principal or verified connection context, plus a
   binding to the sponsoring user's app account?
2. If caller-info is the intended mechanism, can the connection omit it and
   call the same canister using the user's ordinary authority? We need missing
   or stripped context to fail closed, rather than be interpreted as a human
   session. A client-supplied `is_agent` argument would not enforce that.
3. Can the connector obtain only the restricted app credential, without also
   retaining a route to the user's unrestricted app identity? Would this be an
   imcp2 extension, an II change, an existing standard we should adopt, or a
   protocol change?
4. What is the recommended revocation boundary? We understand that already
   issued delegations remain usable until expiry; the related
   [imcp2 issue 121](https://github.com/dfinity/imcp2/issues/121) discusses that
   window. An application-visible connection binding would also let the app
   check its own current grants on every request.

We can implement the folder permission evaluator in the application. We are
looking for the smallest supported identity integration, so each ICP app does
not need to design its own authentication system. For our alpha demo we will use
ICP MCP with the user's full application authority and state that limitation
explicitly, while investigating this extension.
