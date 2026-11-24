# Divider v0.1.45 first PR browser job

The first [PR browser job](https://github.com/faberline/surface/actions/runs/45472903154/job/138725341937) used candidate `d817ae4194f0dca6d81e5fadc895bf58585fbc92` and PR checkout `b20351038f906cbb69a74afceeed5ee00c1a26cc` on source tree `d512db1b4bc02b2d82ec738f304c7be3dabc592c`.

The pinned Binaryen 133 archive download returned HTTP 504. `curl` exited with code 22 and the browser test step was skipped. GitHub recorded no artifact ZIP. The workflow log excerpt and exact job step status show this setup failure. The failure supplies no Divider browser verdict.

To retrieve the complete original GitHub log while it is available, run:

```sh
gh api repos/faberline/surface/actions/jobs/138725341937/logs > attempt-1-browser-e2e.log
shasum -a 256 attempt-1-browser-e2e.log
```

Compare that checksum with `original_job_log_sha256` in `bootstrap-failure.json`. Verify this archive with `shasum -a 256 -c SHA256SUMS`.
