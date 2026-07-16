<!-- ontopolis-machine-data:capability-audit:v1 -->
```json
{
  "capabilities": [
    {
      "authoritative_state": {
        "path": null,
        "rationale": "observer-only",
        "symbol": null
      },
      "capability_class": "state",
      "capability_id": "cap-space-001",
      "carriers": [
        "geometry"
      ],
      "domain": "space",
      "levels": {
        "M0": {
          "evidence_ids": [],
          "rationale": "ok",
          "status": "satisfied"
        },
        "M1": {
          "evidence_ids": [],
          "rationale": "ok",
          "status": "satisfied"
        },
        "M2": {
          "evidence_ids": [],
          "rationale": "ok",
          "status": "satisfied"
        },
        "M3": {
          "evidence_ids": [],
          "rationale": "ok",
          "status": "satisfied"
        },
        "M4": {
          "evidence_ids": [],
          "rationale": "ok",
          "status": "missing"
        },
        "M5": {
          "evidence_ids": [],
          "rationale": "ok",
          "status": "missing"
        }
      },
      "mutation_owner": {
        "path": null,
        "rationale": "none",
        "symbol": null
      },
      "representative_workload": {
        "bounded_inputs": [
          "chunk"
        ],
        "duration": 1,
        "metrics": [
          "coverage"
        ],
        "name": "space-audit",
        "sample_count": 1,
        "status": "present",
        "validation_envelope": [
          "exact_path"
        ],
        "warmup": null
      },
      "target_maturity": "M3"
    }
  ],
  "domains": [
    {
      "capability_ids": [
        "cap-space-001"
      ],
      "domain": "space"
    }
  ],
  "evidence": [],
  "extensions": {},
  "run_id": "audit-26026fb3862e-20260713T215215Z",
  "schema_version": 1,
  "source_baseline_sha": "26507f0c380a943488a2ab0928b5eef77d1f3ca8",
  "source_reconciliation": {
    "evidence_ids": [],
    "rationale": "missing evidence",
    "status": "present"
  },
  "todo_bindings": [
    {
      "acceptance": "contracts",
      "clause_hash": "0000000000000000000000000000000000000000000000000000000000000000",
      "dependency_range": "1-9",
      "goal": "inventory",
      "heading": "Maturity audit",
      "todo_id": "TODO-DEPTH-001"
    }
  ]
}
```
<!-- /ontopolis-machine-data -->
