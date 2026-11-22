import ApplicationServices
import Foundation

func attribute(_ element: AXUIElement, _ key: String) -> Any? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, key as CFString, &value) == .success ? value : nil
}

func children(_ element: AXUIElement, _ key: String) -> [AXUIElement] {
    attribute(element, key) as? [AXUIElement] ?? []
}

func report(_ value: [String: Any]) {
    guard let encoded = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]) else {
        return
    }
    FileHandle.standardOutput.write(encoded + Data([0x0A]))
}

let args = CommandLine.arguments
guard args.count >= 3, let pid = pid_t(args[1]), pid > 1 else {
    fatalError("Usage: read-badge-ax PID LABEL [LABEL ...]")
}
guard AXIsProcessTrusted() else {
    report(["trusted": false])
    exit(0)
}
let requested = Set(args.dropFirst(2))
let app = AXUIElementCreateApplication(pid)
AXUIElementSetMessagingTimeout(app, 0.5)
_ = attribute(app, "AXFocusedUIElement")
let windows = children(app, "AXWindows")
var matches: [[String: Any]] = []
var visits = 0
func visit(_ element: AXUIElement, depth: Int) {
    visits += 1
    guard depth < 32 && visits < 1400 else { return }
    for key in ["AXTitle", "AXDescription", "AXLabel", "AXValue"] {
        guard let label = attribute(element, key) as? String, requested.contains(label) else { continue }
        var actions: CFArray?
        AXUIElementCopyActionNames(element, &actions)
        matches.append([
            "name": label,
            "role": attribute(element, "AXRole") as? String ?? "",
            "enabled": attribute(element, "AXEnabled") as? Bool ?? false,
            "focused": attribute(element, "AXFocused") as? Bool ?? false,
            "actions": actions as? [String] ?? []
        ])
        break
    }
    for child in children(element, "AXChildren") {
        visit(child, depth: depth + 1)
    }
}
for window in windows {
    visit(window, depth: 0)
}
report(["trusted": true, "windows": windows.count, "matches": matches])
