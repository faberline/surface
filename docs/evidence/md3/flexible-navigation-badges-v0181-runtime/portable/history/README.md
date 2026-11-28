# Earlier checks

The first badge layout passed the native pixel checks. Its five-character heading clipped in the 320 dp view with 200% text. The corrected heading uses four characters and fits in the same view.

The first focused browser run rejected a 119 dp cell at 599 dp. The layout rounds consecutive edges, so this cell is 119 dp while an adjacent cell can be 120 dp. The updated test uses the measured edges and the 600 dp layout keeps 112 dp cells.
