<!-- ontopolis-machine-data:sequencing:v1 -->
```json
{
  "candidates": [
    {
      "candidate_id": "TODO-SIM-001",
      "target_capability_ids": [
        "cap-space-001"
      ],
      "todo_id": "TODO-SIM-001"
    },
    {
      "candidate_id": "TODO-RUNTIME-001",
      "target_capability_ids": [
        "cap-space-001"
      ],
      "todo_id": "TODO-RUNTIME-001"
    }
  ],
  "edges": [
    {
      "edge_type": "requires",
      "evidence_ids": [
        "e-001"
      ],
      "from": "node-1",
      "rationale": "a",
      "to": "node-2"
    },
    {
      "edge_type": "requires",
      "evidence_ids": [
        "e-001"
      ],
      "from": "node-2",
      "rationale": "b",
      "to": "node-1"
    }
  ],
  "extensions": {},
  "nodes": [
    {
      "evidence_ids": [
        "e-001"
      ],
      "node_id": "node-1",
      "node_kind": "candidate",
      "status": "selected"
    },
    {
      "evidence_ids": [
        "e-001"
      ],
      "node_id": "node-2",
      "node_kind": "candidate",
      "status": "selected"
    }
  ],
  "run_id": "audit-26026fb3862e-20260713T215215Z",
  "schema_version": 1,
  "selection": {
    "checker_version": "1",
    "minimal_remediation_node_ids": [
      "node-1"
    ],
    "mode": "sim",
    "rationale": "invalid cycle",
    "ready_candidate_ids": [
      "TODO-SIM-001",
      "TODO-RUNTIME-001"
    ],
    "selected_candidate_id": "TODO-SIM-001",
    "tie_break_serialization": "cycle"
  },
  "source_baseline_sha": "26507f0c380a943488a2ab0928b5eef77d1f3ca8"
}
```
<!-- /ontopolis-machine-data -->
