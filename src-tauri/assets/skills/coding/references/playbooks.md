# Playbook file reference

Run `scripts/playbook.py --pick` before planning. It prints the active profile and provider order,
one line per `pickOrder` entry, and each playbook's id, name and `pickWhen`. Then
`scripts/playbook.py <id>` prints only that playbook, the resolved tiers (provider and tier to
model and effort), provider order, quota and budget rules. Both accept `--json`.

The user's copy (`%APPDATA%\CodingOpus\coding-opus-playbooks.yaml`, or `~/.config/coding-opus/`)
replaces the bundled file. `playbook.py --path` prints the file in use and `playbook.py --check`
validates it.

## See and edit the playbooks

Coding Agent edits the playbooks in **Preferences ▸ Playbooks**, a flow view of every playbook
with its conditions and the model each step resolves to, and saves the user's copy. Point the user
there; do not edit the YAML by hand unless asked, and run `playbook.py --check` after any edit.

## Top level

| Field | Meaning |
|---|---|
| `setup` | Written by onboarding: the `date` the user approved the profiles and `notes` on what was found or skipped |
| `principles` | Saving rules that apply to every plan |
| `providers` | The CLIs the user approved, in the base order of preference |
| `models` | Model ids each CLI listed, optionally as `{ id, cost }` with the CLI's cost multiplier |
| `adapters` | A checked custom adapter for each provider without a built-in one |
| `routes` | Gateways on the model traffic path, such as `router9` |
| `tiers` | What `cheap`, `standard` and `strong` mean per provider: a model and an effort |
| `quota.criticalThreshold` | Percent of any main usage window that counts as low on tokens |
| `quota.stopAt` | Percent at which `run` refuses a wave and skips queued workers on that provider; at least `criticalThreshold`, 95 by default |
| `quota.whenLow` | `switch-provider` runs a role on the next provider in the order at the same tier; `keep` stays |
| `quota.onBothCritical` | `ask`, `run-cheapest` or `block` when every provider in the order is low |
| `quota.whenSpare` | Only for subscription billing (`billing` defaults to `subscription`), never API billing: what to do with spare quota: `spend-strong` runs workers on its strong tier, `prefer` runs them on it at their own tier, `ignore` changes nothing |
| `quota.spareWithinHours` | A weekly or monthly cycle that resets within this many hours counts as spare |
| `quota.spareMinLeft` | Percent of that cycle that must be left for it to count as spare |
| `quota.planEnds` | Dates a plan ends without renewing, by provider, for example `{ codex: "2026-10-20" }` |
| `budget.warnAt` | Percent of the plan budget that counts as low |
| `budget.whenLow` | `finish-and-report`, `downgrade-tier` or `stop` |
| `passive` | Optional passive-mode checks and routing preferences; defaults below |
| `profile` | The active profile |
| `profiles` | Named sets of overrides, usually `base`, `savings` and `aggressive` |
| `pickOrder` | Which playbook to pick for which kind of task, checked top to bottom |
| `playbooks` | The playbooks, by id |
| `routing` | Optional; maps one delegation's kind and effort to a role, exported by the UI |

## Passive settings

`passive` is optional and independent of the active profile. Missing fields use these defaults:

```yaml
passive:
  checkEveryMinutes: 15
  spendStepUsd: 5
  prefer: [codex, claude]
  askAboveWorkers: 2
```

`checkEveryMinutes` and `spendStepUsd` must be positive finite numbers. A check is due after
that many minutes since the last printed check, when the lead's list-price estimate crosses a
new spend step, or when a known provider becomes low/blocked or resets. `prefer` is a nonempty
ordered list of distinct configured provider ids; honor billing and quota before routing.
`askAboveWorkers` is a positive integer: ask before exceeding it with parallel workers.
In Coding Agent the lead applies these between requests from the `fitness` block; there is no
background hook.

## Providers

`claude`, `codex`, `agy` and `kimi` have built-in adapters. Any other CLI, such as `kiro`,
`copilot` or `gemini`, runs as a `custom` worker built from its entry in `adapters`:

| Field | Meaning |
|---|---|
| `command`, `args` | The executable and the arguments before the prompt |
| `promptTransport`, `outputFormat` | As in a custom worker: `stdin` or `argument`; `text`, `json` or `jsonl` |
| `modelFlag`, `effortFlag` | The flags that take a model id and an effort, when the CLI has them |
| `checked` | The help command that confirmed the invocation, and when |

A custom adapter cannot enforce read-only work, so `auto` never sends it a `readOnly` phase. Add
the tier's model and effort through its flags when building the worker.

A route carries a CLI's API traffic; it is not a worker. `routes.router9` records the `port`,
whether it is `running`, and which CLIs it `carries`: only `claude` and `codex` speak its dialects.
Model names for a carried CLI go to whatever the gateway maps them to. Never start, install or
reconfigure a route; Coding Opus does that.

## Profiles

The active profile changes three things and leaves the playbooks alone:

| Field | Effect |
|---|---|
| `order` | Replaces the order of `providers`; providers it leaves out are tried last |
| `quota` | Overrides those `quota` fields |
| `tiers` | Overrides the model and effort of the tier and provider entries it names |

`base` usually has no overrides. Switch profiles by changing `profile`, after confirming when it is
an ongoing change rather than one run. All three tiers (`cheap`, `standard`, `strong`) resolve from
`tiers` plus the active profile's overrides. Deprecated `policy.modelTiers` overrides still win
when set; `policy set --cheap-model` is a legacy path, not where onboarding writes new tiers.

## Phases

A playbook lists `phases`. Each phase is one role:

| Field | Meaning |
|---|---|
| `id` | Unique within the playbook |
| `role` | The name used in briefs and shown to the user |
| `who` | `lead` is you, in this session; anything else is a worker wave |
| `tier` | `cheap`, `standard` or `strong`, resolved through the active tiers |
| `provider` | `auto` (default) takes the first provider in the active order that can run the phase; a provider id pins it |
| `model`, `effort` | Override the tier for this phase |
| `readOnly` | Run workers with `mode: read-only` |
| `workers` | Units in the wave, within the app's concurrency limit (Preferences ▸ Orchestration) |
| `isolate` | Give each writer its own worktree |
| `runner` | Legacy runner strategy (`economy`, `balanced`, `quality`, `research`, `parallel` or `rescue`); kept for compatibility, Coding Agent ignores it |
| `does`, `returns`, `limit` | What to put in each brief and what to ask back |
| `gate` | What you check after the phase before moving on |
| `if` | The phase runs only when the condition holds |
| `with` | Joins that phase's step. When any path in the step has `if`, the paths are alternatives and the first whose `if` holds runs; when none has `if`, they all run in the same wave |
| `after` | Runs only after the `gate` of that phase fails, or after that escalation phase |
| `then` | `next` (default), `recheck` (check the gate again) or `done` (stop here) |
| `rules` | Model choices for this role, checked top to bottom: `if` a condition, `use` a tier, provider, model or effort |

## Choosing the model

For each worker:

1. Start from the phase's tier and provider, with the active profile's tiers and order.
2. Apply the first rule whose `if` holds. Skip a rule whose model belongs to a different provider
   unless it also names that provider. A rule can steer a phase off expensive models with
   `price-above` and cap a run's later waves with `spent-above`, for example
   `if: price-above:5` → `use: { tier: cheap }`.
3. Before every delegation, read the `fitness` block that `coding_agent_status` (and every other
   `coding_agent_*` response) returns. If the chosen provider is low, move to the next capable
   provider in the active order at the same tier, resolving that provider's model and effort.
   Respect a provider the user pinned; a playbook's suggested provider does not prevent adapting
   the plan. Never lower a tier to dodge a low provider; ask the user instead.
4. If every provider is low, follow `quota.onBothCritical`.
5. Apply `quota.whenSpare` only to subscription-billed providers when unused quota would be lost,
   under every goal and profile. Check `billing` in `coding-opus-prices.json` (default
   `subscription`); API billing has no expiring quota and never triggers this rule. Keep the default
   `spend-strong`. Goal `save` is not required. Keep budget limits and user provider pins; when
   several subscription providers have spare quota, prefer the one that loses it first.

Change the plan before delegating; these are lead decisions. A provider at or past `quota.stopAt`
is out: its workers end as `succeeded` with nothing done. Tell the user at most one quota line per
delegation with the reading and reset time. Say when a reading is stale rather than asserting its
number.

Pass the resolved model and effort explicitly in `coding_agent_delegate` (or a role from the app's
Orchestration settings). If a model stays empty, the CLI uses its default model, which can be the
most expensive one.

## Usage windows

| Provider | Main windows | Cycle used for spare quota |
|---|---|---|
| `claude` | `5h`, `week` (plus `opus`) | week |
| `codex` | `5h`, `week` | week |
| `agy` | `gemini`, `claude` (plus `gpt`) | none |
| `kimi` | `5h`, `week` (plus `month`) | week |
| `kiro` | `credits` | monthly credits |
| `copilot` | `premium` | monthly premium requests |
| `gemini` | `day` | none |

Coding Agent polls the provider usage and returns it as the `fitness` block of every
`coding_agent_*` response: for each provider the worst window, `used` percent, `resetsAt` and
`rateLimited`. Use the active profile's thresholds: `low` at or above `quota.criticalThreshold`,
out at or above `quota.stopAt`. Within a window usage only grows, so a stale reading still blocks
until its `resetsAt` passes. Claude's `opus` window blocks only Opus workers. Missing data is
unknown, not evidence of spare quota.

## Spare quota

A provider's quota is spare when it is not low, at least `spareMinLeft` percent of its cycle is
left, and that quota would be lost: its plan ends before the cycle resets, or the cycle resets
within `spareWithinHours`. A plan that ends in 2 days while the week resets in 6 loses everything
left. For subscription billing, spending that quota does not add an API charge; API billing still
incurs the model's price. Prices and billing per model live in the app's `prices.json`.

## Conditions

`if` takes one condition or a list; a list holds when all of them hold. Prefix one with `not ` to
negate it. `<provider>` is any provider id and `<window>` one of its windows above.

| Condition | Holds when |
|---|---|
| `narrow` | The task is small and clear, up to about 3 files |
| `risky` | The unit changes an API contract, a database schema or migration, authentication, cryptography or secrets, or deletes data. Judge it by the operation, not by a word such as "auth" in the brief |
| `kind:<kind>` | The unit's kind is `research`, `code`, `review`, `command`, `scrap` or `docs` |
| `light`, `standard`, `deep` | The unit's effort class |
| `<provider>-low` | A fresh `usage` main window is at or above `criticalThreshold`, it is listed in `conserveProviders`, or Coding Opus or the user reports it low |
| `all-low` | Every provider in the order is low; `both-low` means the same |
| `<provider>-above:<n>` | The busiest main window in a fresh reading is past n percent |
| `<provider>-<window>-above:<n>` | That window in a fresh reading is past n percent, for example `claude-opus-above:50` or `codex-week-above:80` |
| `<provider>-spare` | That subscription-billed provider has fresh spare quota; false for API billing |
| `budget-low` | More than `budget.warnAt` percent of the plan budget is spent |
| `price-above:<usd>` | The model the unit would run on, from the phase's tier, provider and model before this rule, lists an input price above `<usd>` US dollars per 1M tokens in the price table, for example `price-above:5` or `price-above:2.5`. A model with no known price, including the CLI's default when none is pinned, cannot establish it |
| `spent-above:<usd>` | This orchestration's reported spend so far, as the Coding Agent board totals it, is above `<usd>` US dollars. Only cost the CLIs reported counts; nothing is estimated |
| `checks-failed`, `checks-failed-twice` | The phase's checks failed once, or twice |
| `report-weak` | A report lacks the evidence it was asked for |
| `conflict` | Reports contradict each other |
| `stuck` | A worker stopped at a decision it could not make |
| `no-pass` | No attempt passed |
| `findings-left` | Confirmed review findings remain |

In skill-only mode Claude and Codex quota conditions use fresh `usage` readings. Their extra
windows apply only when reported. For Antigravity and Kimi, quota conditions still need Coding Opus or
the user; for other providers, ask the user. A stale or absent reading cannot establish an
`above` or `spare` condition. `conserveProviders` and explicit user warnings still establish low.
Spare quota requires subscription billing (`billing` defaults to `subscription`), never API billing,
and also needs a reset time; a non-renewing plan end comes from `quota.planEnds` or the
user. Ignore a `planEnds` date that has passed. Task, check and report conditions are judged by
the lead from evidence; budget conditions, `spent-above` among them, use the spend the board reports.
`price-above` reads the price table `models` shows: the bundled `prices.json` overlaid by
`coding-opus-prices.json` next to the policy, keyed `provider/model`. It is a planning choice the lead
makes before the wave.
