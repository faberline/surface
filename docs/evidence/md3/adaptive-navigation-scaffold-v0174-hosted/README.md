# Adaptive navigation scaffold candidate CI

The original push and PR workflows checked the same product tree.
The source push used `0b84d56fe9990e15908165157d8d9bc6f7709223`.
The PR checkout used `a39a739bf5bbc041472647f3f15822bca71b1c90`.
The reviewed tree is `2532449d43fdce54448a5535cb6fcd5cb5ff6a79`.

Both original GitHub artifact ZIPs matched the digest and byte count in their download receipts.
The packages preserve the receipts, original logs and PNG subset. The complete ZIPs are omitted.

The push workflow passed 1,580 Rust tests and three Wasm checks.
It passed 31,107 browser and bridge checks, including 368 Material steps and
30,449 Material checks. [Original GitHub workflow](https://github.com/faberline/surface/actions/runs/46165187549)

```sh
python3 -E -B push/hosted.py --verify push --config-sha 954dbe3f0b41a9312ba2b1f459aa1c9560e2a37d19bc4ca9c988a9ec651d29bc --manifest-sha ed467917e60ae3e0dee5e004955a7bb0d8de85cbd0d7697bb9f95da156c3401f
```

The pr workflow passed 1,580 Rust tests and three Wasm checks.
It passed 31,107 browser and bridge checks, including 368 Material steps and
30,449 Material checks. [Original GitHub workflow](https://github.com/faberline/surface/actions/runs/46165195113)

```sh
python3 -E -B pr/hosted.py --verify pr --config-sha 1c6c42e5473a325280bec52852a074dd361d6ee4998f8eb4beb95b2aafff086d --manifest-sha e58f5f019a5643ede6769fdf041d4951ed20c1265f78872e2f00c011f929a985
```

Use Python 3.12 and Pillow for the offline verification commands.
