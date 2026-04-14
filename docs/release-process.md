# Release process

1. Freeze a spec revision.
2. Regenerate test vectors.
3. Require passing CI on Python and Rust.
4. Tag a release candidate.
5. Run testnet soak and load tests.
6. Commission or update an external audit against the exact tag.
7. Fix findings.
8. Re-run CI, testnet soak, and release-candidate verification.
9. Sign release artifacts.
10. Publish the release notes and compatibility matrix.
