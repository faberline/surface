# Selection after a canceled composition

Surface v0.1.42 candidate `ab8615c8495d99a652f4fac38d4db37f08c82c7f` stays in draft. The same Chrome 158 headless replay reproduces a selection rollback in this candidate and in the staged v0.1.43 source. The published v0.1.41 bundle passes the controlled comparison.

The user finishes editing 台文, starts the preedit ㄅㄆ, then cancels composition. A trusted Select All selects both characters in the textarea. The diagnostic pauses the animation clock only to require a real 1000 to 1001 dp Canvas redraw between selection and text replacement. It records the browser range in UTF-16 units and Surface's selection in UTF-8 bytes. The traces also include normal timing without a forced redraw.

| Bundle | Result | Controlled | Normal timing |
| --- | --- | --- | --- |
| Surface v0.1.41 | Browser range 0–2 and Runtime range 0–6 survive the redraw. The next text replaces the whole value. | [Trace](v041-controlled.json) | |
| Surface v0.1.42 draft | The browser selects 0–2. Runtime keeps 3–3. Redraw puts the browser back at 1–1. The new text is inserted into 台文. | [Trace](v042-controlled.json) | [Failing trace](v042-natural.json) |
| Surface v0.1.43 staged Search | The same rollback occurs in the original Showcase field outside Search. | [Trace](v043-controlled.json) | [Failing trace](v043-natural.json) |

The staged v0.1.43 full check first failed stock Showcase step I11. A second full run passed. The controlled replay proves the timing-sensitive regression on each affected bundle even when broad smoke checks pass. Keep both release candidates on hold until the corrected head passes the reproduction and release gates.

[Candidate PR](https://github.com/faberline/surface/pull/217)
