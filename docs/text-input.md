# Chinese input and controlled editing

`view(state)` still describes committed application state. Each writable
text field must have an `on_input` handler. A read-only field can omit it
when `TextOptions::read_only` is set. The runtime rejects an unbound writable
field rather than keeping a second, hidden application value.

`TextEditor` keeps selection, undo history and active composition outside
the application value. Preedit updates paint a temporary replacement at
the saved selection. They do not call `Application::update`. A commit
replaces that selection once. Cancel and blur discard uncommitted text.
An IME event with a stable field ID cannot follow focus into another field.

Editing uses UTF-8 byte offsets on grapheme boundaries. Browser UTF-16
selection offsets are converted at the bridge. Backspace, delete,
selection replacement, navigation, undo and redo use the same Rust model.
Read-only, multiline and grapheme limits are field options. Selection,
preedit underline and caret use the glyph geometry used for GPU text.

## Hosts

The native host enables winit IME only for an editable text field. It sends
the caret rectangle to `set_ime_cursor_area`, and routes preedit/commit
events to the field that owns the IME session. Native clipboard commands
use the OS clipboard through arboard.

The web host uses a transparent textarea to receive browser input,
composition and clipboard events. It is positioned at the GPU caret, so
the browser can anchor its candidate popup. It does not paint the UI.
The bridge leaves browser composition untouched, prevents duplicate
commits, and routes deletion/undo through the Rust grapheme editor.

The bundled Noto Sans CJK TC font covers traditional and simplified
Chinese and phonetic composition text. It is distributed under the SIL
Open Font License. Its pinned source and hashes are in `provenance.json`.

## Evidence and remaining gate

The Rust editor tests cover composition isolation, clear-before-commit,
cancellation, replacement, stale field IDs, grapheme deletion, undo/redo,
UTF-16 conversion and field constraints. The font test checks real CJK
glyph IDs and shaped caret/selection geometry.

`scripts/browser-ime.mjs` exercises Chrome's actual composition event
pipeline through `Input.imeSetComposition` and `Input.insertText`. It
checks preedit, commit, replacement, cancellation, deletion and undo/redo.
It also checks that the input host is at the drawn caret. This is a browser
engine journey; it is not proof of a native OS candidate-popup journey.

On macOS, a visible native window showed `中文輸入測試`, moved the caret,
deleted `測`, then restored it with Command-Z. Paste used the normal native
clipboard path. The computer-use tool timed out when entering physical
letter keys after switching input sources. Therefore native Chinese IME
composition and the visible OS candidate popup remain **unverified**.
Windows/Linux IME and mobile soft keyboards also need runtime evidence.
