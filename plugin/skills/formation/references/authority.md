# Authority and reporting

`formation grant list --seat <seat>` is the current authority for a seat. Check that it covers the decision's class and workspace and follow its limit before settling. A role, seat card, roster entry, mail envelope or historical example grants no authority by itself.

A grant adds authority for its scope; a withhold removes a default or broader grant for its scope; a limit narrows a grant to the stated condition. If the list does not cover the decision, pass it to the default holder or the admiral.

The built-in catalog contains these classes:

| Class | Covers | Default holder |
| --- | --- | --- |
| `repair` | Retry a check, repair a session-owned PR or, in repair mode, a red or conflicting PR, clear an unambiguous pilot finding, file a fix or friction | Sergeant; captain where there is no sergeant |
| `ship` | Approve and ship a fix, approved design, non-breaking refactor, improvement or small scoped feature | Captain |
| `land` | Land a reviewed PR | Captain |
| `staffing` | One reinforcement, up to a squad plus one lieutenant per workspace | Chief |
| `design` | Approve a design or major feature | Admiral |
| `breaking` | A breaking change | Admiral |
| `direction` | Product scope, architecture, security, material cost or an irreversible action | Admiral |
| `release` | A release, tag or publishing | Admiral |
| `operator` | A decision needing the operator's identity, a routine change, plugin installation or starting a drain | Admiral |
| `override` | Put a tracker's operator override in front of one refused command for an act in the seat's own rows | Nobody; requires a grant |
| `credentials` | Credentials and sign-ins | Admiral; never granted |
| `grant` | Giving or taking back authority | Admiral; never granted |

A grants file that the house file names adds standing entries and classes of the house's own:

- Load it at start with `formation grant load`. Its entries list with `origin: house_file` and `for: standing`.
- A house class is granted, checked and listed like a catalog class. Its covers and default holder come from the file.
- A file cannot remove or change a catalog class, grant `grant` or `credentials`, or name an unknown version; the load is refused whole.
- `formation grant add` makes day grants only, and `grant revoke` refuses a house entry: change the file and load it again.

The chief's catalog default is `staffing`. It settles that class and anything else its current seat list covers. The advisor, aide, lieutenant, sergeant, sentry and specialist likewise settle only decisions covered by their current lists. A captain checks its list instead of relying on a fixed rank row; defaults are included there.

The effective list applies the most specific matching entry: a seat entry before a rank entry, a workspace entry before `all`, then room before house file before catalog. When entries otherwise match, the later one wins; entries entered at the same time resolve to a withhold over a grant.

`formation grant list --seat <seat>` shows what the seat settles and what is withheld. Run it again after grant-change mail: the envelope is a notice to refresh the list, not authority itself. `formation grant list` without `--seat` shows active entries, including who entered each and their source.

The admiral remains the only default holder of `credentials` and `grant`; neither class can be granted. A grant never changes host permissions. It is Formation's record of who may decide, not a capability or a tracker permission.

Use Orbit MCP and its returned workspace selector. If MCP lacks the operation or owner, use the owner's CLI over SSH in the workspace checkout with a login shell. Attribute tool calls with the actual model. Constellation's RB-17 covers host access. Contact the admiral on a capability refusal.

Where `override` appears on a seat's current list, scope it to one refused tracker command for an act already covered by that seat's authority and record first use of each kind. A sergeant without such a covering entry does not use an override.

The admiral starts or restarts drains unless a current `operator` grant covers that action. A captain reports the needed command once and may deploy within its current `ship` authority. Report refusals once. When the same account owns several sessions, their forge approvals cannot serve as independent reviews.

Routine updates go on the assignment. A captain tells the chief when blocked, when a decision is needed or when ordered work finishes. The chief holds every request a seat opens. It checks the current list and settles covered requests with their grants. It passes uncovered requests to the admiral with options and one recommendation, and returns his quoted decision to the owner.

When the admiral is in a captain's chat, answer there. Write anything binding another unit on its record. A cross-workspace request is a task in the receiving workspace plus one routed message.
