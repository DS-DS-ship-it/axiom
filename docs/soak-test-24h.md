# AXIOM 24-Hour Soak Test

## Summary

AXIOM completed a 24-hour soak test on a 4-node local validator setup with:

- periodic restart cycling
- malformed message stress
- oversized message stress
- continuous sampling of process health

The run completed with **0 recorded failures**.

## Result

- Duration: **24.0 hours**
- Samples: **5621**
- Planned restarts: **287**
- Failures: **0**
- Base port: **7301**

## Interpretation

This soak test indicates that the current AXIOM validator prototype was able to remain operational for a full 24-hour run under repeated restart and message-abuse conditions without recorded harness failures.

This does **not** by itself prove mainnet readiness, formal security, or correctness under all byzantine conditions. It is an operational stability result for the current pre-audit prototype.

## Raw summary output

```json
{
  "out_dir": "/var/folders/kh/_jfld9zx3j78t5fjj9qc2v4m0000gn/T/axiom-soak-7crkc7e2",
  "report_path": "/var/folders/kh/_jfld9zx3j78t5fjj9qc2v4m0000gn/T/axiom-soak-7crkc7e2/soak_report.jsonl",
  "samples": 5621,
  "restarts": 287,
  "failures": 0,
  "duration_hours": 24.0,
  "base_port": 7301
}
