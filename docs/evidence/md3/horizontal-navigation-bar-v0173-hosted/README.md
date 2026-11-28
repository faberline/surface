# Horizontal flexible navigation bar candidate CI

The original push and PR workflows checked the same product tree.
The source push used `c2571a3d818ddaf0cb7874998d1c39f0ffc7b1f2`.
The PR checkout used `94c453784ceba6a1d2545f95e051c692b7dd1572`.
The reviewed tree is `791d8b2dd3f9b44f3c2a967f1418b671920a2899`.

Both original GitHub artifact ZIPs matched the digest and byte count in their download receipts.
The packages preserve the receipts, original logs and PNG subset. The complete ZIPs are omitted.

The push workflow passed 1,575 Rust tests and three Wasm checks.
It passed 30,907 browser and bridge checks, including 364 Material steps and
30,249 Material checks. [Original GitHub workflow](https://github.com/faberline/surface/actions/runs/46156800398)

```sh
python3 -E -B push/hosted.py --verify push --config-sha 4331abc46eb0bbeb5771918e04dbda6d076335e4f0569ea0478de6411b4a891b --manifest-sha 07af43e68de629e3d6e2652f2df8f15ca073aa079752201aebd61cfe1c7cd0de
```

The pr workflow passed 1,575 Rust tests and three Wasm checks.
It passed 30,907 browser and bridge checks, including 364 Material steps and
30,249 Material checks. [Original GitHub workflow](https://github.com/faberline/surface/actions/runs/46156806154)

```sh
python3 -E -B pr/hosted.py --verify pr --config-sha 8ac5d72abf0d7e57d0ad3032830a3fa1ef7a1f3b8c5edcd2e42ac6feef0b1f73 --manifest-sha 2ec0789a46751b72984dcb8e2e781d0847ae3fe88e2ffee41918e5c1b2a562b8
```

Use Python 3.12 and Pillow for the offline verification commands.
