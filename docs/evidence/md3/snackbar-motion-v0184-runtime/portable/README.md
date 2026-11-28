# q84 Snackbar runtime evidence

Source b067e40910e9f86683ef6352ec631e9c8a8436dc, tree 34f4de1d361c5693643059da0d6c602732ace4a8, 718 files. The original stock Browser run passed 391 ordered Material steps. It includes two independent native runs, Page.screenshot/CLI/MCP parity, keyboard dismissal and focus return, original PNGs, Metal logs, and headless process/window receipts.

Native screenshots show settled states, and the raw receipts record the bounds sampled before and after dismissal. The browser screenshots contain actual intermediate exit paint. VoiceOver speech and physical-display behavior did not run.
