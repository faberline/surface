import ApplicationServices
import Foundation

func attribute(_ element: AXUIElement, _ key: String) -> Any? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, key as CFString, &value) == .success ? value : nil
}

func children(_ element: AXUIElement, _ key: String) -> [AXUIElement] {
    attribute(element, key) as? [AXUIElement] ?? []
}

func report(_ value: [String: Any]) throws {
    let encoded = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    FileHandle.standardOutput.write(encoded + Data([0x0A]))
}

let args = CommandLine.arguments
guard args.count >= 3, let pid = pid_t(args[1]), pid > 1 else {
    fatalError("Usage: read-drawer-ax PID LABEL [LABEL ...] | PID --press LABEL")
}
guard AXIsProcessTrusted() else {
    try report(["trusted": false])
    exit(0)
}
let press = args[2] == "--press"
let requested = Set(args.dropFirst(press ? 3 : 2))
let app = AXUIElementCreateApplication(pid)
AXUIElementSetMessagingTimeout(app, 0.5)
_ = attribute(app, "AXFocusedUIElement")
let windows = children(app, "AXWindows")
var matches: [[String: Any]] = []
var action: Int32?
var visits = 0

func visit(_ element: AXUIElement, depth: Int) {
    visits += 1
    guard depth < 32 && visits < 1400 else { return }
    let role = attribute(element, "AXRole") as? String ?? ""
    let names = ["AXTitle", "AXDescription", "AXLabel"].compactMap { attribute(element, $0) as? String }
    if role == "AXButton", let name = names.first(where: { requested.contains($0) }) {
        var actions: CFArray?
        AXUIElementCopyActionNames(element, &actions)
        var row: [String: Any] = [
            "name": name,
            "role": role,
            "enabled": attribute(element, "AXEnabled") as? Bool ?? false,
            "focused": attribute(element, "AXFocused") as? Bool ?? false,
            "actions": actions as? [String] ?? []
        ]
        if let current = attribute(element, "AXARIACurrent") as? String {
            row["current"] = current
        }
        matches.append(row)
        if press, action == nil {
            action = AXUIElementPerformAction(element, kAXPressAction as CFString).rawValue
        }
    }
    for child in children(element, "AXChildren") {
        visit(child, depth: depth + 1)
    }
}

for window in windows {
    visit(window, depth: 0)
}
var result: [String: Any] = ["trusted": true, "windows": windows.count, "matches": matches]
if let action {
    result["press_status"] = action
}
try report(result)
