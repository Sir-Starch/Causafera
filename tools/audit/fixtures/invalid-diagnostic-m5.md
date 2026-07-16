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
          "evidence_ids": [
            "e-001"
          ],
          "rationale": "ok",
          "status": "satisfied"
        },
        "M1": {
          "evidence_ids": [
            "e-001"
          ],
          "rationale": "ok",
          "status": "satisfied"
        },
        "M2": {
          "evidence_ids": [
            "e-001"
          ],
          "rationale": "ok",
          "status": "satisfied"
        },
        "M3": {
          "evidence_ids": [
            "e-001"
          ],
          "rationale": "ok",
          "status": "satisfied"
        },
        "M4": {
          "evidence_ids": [
            "e-001"
          ],
          "rationale": "ok",
          "status": "satisfied"
        },
        "M5": {
          "evidence_ids": [
            "e-001"
          ],
          "rationale": "ok",
          "status": "satisfied"
        }
      },
      "mutation_owner": {
        "path": null,
        "rationale": "none",
        "symbol": null
      },
      "representative_workload": {
        "bounded_inputs": [
          "timer"
        ],
        "duration": 1,
        "metrics": [
          "time"
        ],
        "name": "diagnostic",
        "sample_count": 1,
        "status": "present",
        "validation_envelope": [
          "diagnostic-only"
        ],
        "warmup": 0
      },
      "target_maturity": "M5"
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
  "evidence": [
    {
      "adapter": "benchmark_diagnostic",
      "blob_oid": "0000000000000000000000000000000000000000",
      "command": "benchmark-diagnostic",
      "evidence_id": "e-001",
      "exit_code": 0,
      "extensions": {},
      "facets": [
        "validation_envelope"
      ],
      "line_end": 1,
      "line_start": 1,
      "path": "tools/audit/fixtures/invalid-diagnostic-m5.md",
      "receipt_path": "tools/audit/fixtures/invalid-diagnostic-m5.md",
      "receipt_sha256": "0000000000000000000000000000000000000000000000000000000000000000",
      "run_id": "audit-26026fb3862e-20260713T215215Z",
      "source_baseline_sha": "26507f0c380a943488a2ab0928b5eef77d1f3ca8",
      "symbol": "space",
      "workload": {
        "name": "diagnostic"
      }
    }
  ],
  "extensions": {},
  "run_id": "audit-26026fb3862e-20260713T215215Z",
  "schema_version": 1,
  "source_baseline_sha": "26507f0c380a943488a2ab0928b5eef77d1f3ca8",
  "source_reconciliation": {
    "evidence_ids": [
      "e-001"
    ],
    "rationale": "aligned",
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
