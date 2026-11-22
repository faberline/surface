"""Route the disposable control harness to DN1–4 and the original DN25–27."""
import difflib
from hashlib import sha256
from pathlib import Path
import re

proof = Path(__file__).resolve().parent
file = proof / 'repo/e2e/src/devtools_native.rs'
before = file.read_text()
after, changed = re.subn(
    r'pub const STEPS: &\[&str\] = &\[.*?\];',
    'pub const STEPS: &[&str] = &["DN1", "DN2", "DN3", "DN4", "DN25", "DN26", "DN27"];',
    before,
    count=1,
    flags=re.S,
)
assert changed == 1
first_unrelated_step = '        step("DN5", "DOM.snapshot on the same connection");'
original_helper = '/// Run the client with `--session <file> <args>`.'
assert after.count(first_unrelated_step) == 1 and after.count(original_helper) == 1
begin = after.index(first_unrelated_step)
end = after.index(original_helper, begin)
after = after[:begin] + '''        // Disposable route. Preserve raw authentication and run the unchanged
        // screenshot assertions without assuming the legacy DOM version.
        let screenshot = screenshot_protocol(&mut client, env)?;
        drop(client);
        let tool = env.devtools_bin()?;
        let mut transcripts = String::new();
        let session_arg = devtools_file.to_string_lossy();
        screenshot_cli(&tool, &session_arg, folder, &mut transcripts)?;
        let mcp = mcp_session(&tool, &session_arg, &mut transcripts)?;
        screenshot_mcp(&mcp)?;
        check!(
            !transcripts.contains(&token),
            "the client printed the session token"
        );
        Ok(json!({ "screenshot": screenshot }))
    }
    .await;
    terminate(&mut app).await;
    let evidence: Value = outcome?;
    env.write_json(&format!("{NAME}-result.json"), &evidence)?;
    println!("  evidence: {evidence}");
    Ok(())
}

''' + after[end:]
helper = lambda source: source.split(original_helper, 1)[1]
assert helper(before) == helper(after)
file.write_text(after)
relative='e2e/src/devtools_native.rs'
with (proof/'runner-route.patch').open('w') as sink:
    sink.writelines(difflib.unified_diff(
        before.splitlines(keepends=True), after.splitlines(keepends=True),
        fromfile=f'a/{relative}', tofile=f'b/{relative}',
    ))
print(f"byte-identical helpers SHA-256 {sha256(helper(after).encode()).hexdigest()}")
