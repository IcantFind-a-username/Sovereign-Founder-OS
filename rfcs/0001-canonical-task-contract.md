# RFC 0001: Canonical Task Contract

**Status:** Draft  
**Stage:** 0 → 1

## Summary

Define the JSON schema for enterprise tasks compiled from natural language goals.

## Motivation

Mission Compiler output must be a stable contract between UI, planner, policy engine, and audit ledger.

## Proposed Schema (Draft)

```json
{
  "task_id": "uuid",
  "venture_id": "string",
  "title": "string",
  "stage": "idea_validation | customer_validation | offer | launch | operate",
  "steps": [
    {
      "step_id": "uuid",
      "action": "string",
      "tool": "string",
      "resource": "string",
      "data_class": "red | amber | green",
      "automation_level": "l0 | l1 | l2 | l3",
      "status": "pending | running | waiting_approval | completed | failed"
    }
  ]
}
```

## Open Questions

- How are task dependencies represented?
- Checkpoint format for workflow recovery (Stage 3)?

## Implementation

Types will live in `crates/contracts` as the schema stabilizes.
