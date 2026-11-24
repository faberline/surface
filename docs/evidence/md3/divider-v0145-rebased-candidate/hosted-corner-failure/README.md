# Divider v0.1.45 original SwiftShader corner failures

Both browser jobs used exact candidate `d817ae4194f0dca6d81e5fadc895bf58585fbc92` on tree `d512db1b4bc02b2d82ec738f304c7be3dabc592c` and the Chrome SwiftShader software graphics adapter. Both ran the eight stock suites and reported 18,685 checks. Material stopped at M266. Each build job passed.

- [Push attempt 1](https://github.com/faberline/surface/actions/runs/45472881234/job/138725260508) executed the exact candidate checkout.
- [PR browser attempt 2](https://github.com/faberline/surface/actions/runs/45472903154/job/138730392339) used the exact PR merge checkout and retained the successful build from attempt 1. The earlier PR browser attempt stopped before tests during an HTTP 504 download.

The compact Heavy screenshot and failure screenshot match byte for byte within each job and across both jobs. Each eight-dp line has three partially covered pixels at the lower-right corner. The rest of each line is solid `[73, 69, 78]`, and every adjacent outside edge has the background color. The exact Mac run paints those corner pixels solid. The pinned portable verifier independently checks all interior rows or columns and the adjacent background on the Linux captures.

The Rust check samples the far end one dp from the edge. At 2×, this lands on a lower-right corner pixel with observed color `[65, 61, 70]` in each Heavy line. Moving the endpoint sample to 1.5 dp from each end passes all 96 original samples per job without changing thickness or the required ink color. The renderer’s derivative-based edge coverage accounts for the measured blend.

The original ZIPs match GitHub’s published checksums. The retained raw JSON and PNGs match members of those original ZIPs. Both IME cancellation reports and all 41 Dialog images per job were independently checked before sealing the scoped receipt. Verify the archive and replay the original versus revised probes with Pillow:

```sh
shasum -a 256 -c SHA256SUMS
python3 -B verify-corners.py
```

Retrieve the full ZIPs and logs while GitHub keeps them:

```sh
gh api repos/faberline/surface/actions/artifacts/14961610122/zip > push-artifact.zip
gh api repos/faberline/surface/actions/artifacts/14959972345/zip > pr-artifact.zip
gh api repos/faberline/surface/actions/jobs/138725260508/logs > push-browser-e2e.log
gh api repos/faberline/surface/actions/jobs/138730392339/logs > pr-browser-e2e.log
shasum -a 256 push-artifact.zip pr-artifact.zip
```
