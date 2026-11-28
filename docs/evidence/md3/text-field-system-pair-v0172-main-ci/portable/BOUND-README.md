# Surface v0.1.72 main-run proof

Run this only after PR #248 merges. The binder checks the current main branch and discovers one original `push` run from that exact commit. It requires the squash commit to use tree `8673365398b4c7b51f5e7409abe280d79b79d852` and sole parent `a64f60b796b7bffcd376a99aac903b5235dee2b7`.

First check the 691-file source manifest and review the binding SHA printed by `bind-main.py`.

The bound archive checks 14 system Field/FieldText PNGs, 11 selection PNGs, and 15 disabled-label contrast PNGs from the original artifact.

```sh
python3 -E -B bind-main.py --check-source
python3 -E -B bind-main.py --bind /tmp/surface-md3-v0172-main-binding
```

Then run the collector and review its artifact manifest SHA.

```sh
python3 -E -B /tmp/surface-md3-v0172-main-binding/hosted-main.py --preflight --binding-sha REVIEWED_BINDING_SHA
python3 -E -B /tmp/surface-md3-v0172-main-binding/hosted-main.py --collect /tmp/surface-md3-v0172-main-originals --binding-sha REVIEWED_BINDING_SHA
python3 -E -B /tmp/surface-md3-v0172-main-binding/hosted-main.py --export /tmp/surface-md3-v0172-main-portable --originals /tmp/surface-md3-v0172-main-originals --binding-sha REVIEWED_BINDING_SHA
python3 -E -B /tmp/surface-md3-v0172-main-portable/hosted-main.py --verify-package /tmp/surface-md3-v0172-main-portable --binding-sha REVIEWED_BINDING_SHA --package-sha REVIEWED_PACKAGE_SHA
```
