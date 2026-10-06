# coord-log

The lumen-lang coordinator's event log: one JSON object per line in `events.jsonl`, appended as jobs change
state (started, resumed, finished, reviewed, accepted, returned, handed over, integrated, merged, slots changed).

- Core fields on every line: `t` (UTC time), `event`; `job` where it applies. Other fields vary by event; readers
  ignore fields they do not know. Lines are only appended; a correction is a new `corrected` line.
- `"source":"backfill"` marks lines reconstructed (2026-10-07) from the coordinator's older text logs, 29 Sep onward.
- `legacy-file` / `legacy-line` events hold the coordinator's four retired text logs (astra-log.md, batches.txt,
  telemetry.tsv, reviewed.tsv) verbatim, line by line; `python3 reconstruct-legacy.py <dir>` rebuilds them byte for
  byte and checks each against its recorded sha256.
- Costs and quota readings are recorded here too (Ivan, 2026-10-07): `quota` events (account meters, every check-in),
  `luna-meter` events (OpenAI meter vs our own expected use) and `cost-baseline` events (spend in deleted OpenCode
  sessions, added by the spending caps). Credentials are never recorded.

This branch shares no history with `main` and is never merged into it.
