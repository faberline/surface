import ApplicationServices
import Foundation

func attribute(_ element: AXUIElement, _ key: String) -> Any? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, key as CFString, &value) == .success ? value : nil
}

func elements(_ element: AXUIElement, _ key: String) -> [AXUIElement] {
    attribute(element, key) as? [AXUIElement] ?? []
}

func report(_ value: [String: Any]) throws {
    let encoded = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    FileHandle.standardOutput.write(encoded + Data([0x0A]))
}

let args = CommandLine.arguments
guard (2...3).contains(args.count), let pid = pid_t(args[1]), pid > 1 else {
    fatalError("Usage: read-progress-ax PID [scroll]")
}
let scroll = args.last == "scroll"
guard AXIsProcessTrusted() else {
    try report(["trusted": false])
    exit(0)
}
let app = AXUIElementCreateApplication(pid)
AXUIElementSetMessagingTimeout(app, 0.5)
_ = attribute(app, "AXFocusedUIElement")
let windows = elements(app, "AXWindows")
var nodes: [[String: Any]] = []
var scrollStatus: Int32?
var visits = 0
func visit(_ element: AXUIElement, depth: Int) {
    visits += 1
    guard depth < 32 && visits < 1500 else { return }
    if scroll, scrollStatus == nil {
        var actions: CFArray?
        AXUIElementCopyActionNames(element, &actions)
        if (actions as? [String])?.contains("AXScrollDownByPage") == true {
            scrollStatus = AXUIElementPerformAction(element, "AXScrollDownByPage" as CFString).rawValue
        }
    }
    if attribute(element, "AXRole") as? String == "AXProgressIndicator" {
        let names = ["AXTitle", "AXDescription", "AXLabel"]
            .compactMap { attribute(element, $0) as? String }
        var actions: CFArray?
        AXUIElementCopyActionNames(element, &actions)
        var row: [String: Any] = [
            "name": names.first { !$0.isEmpty } ?? "",
            "actions": actions as? [String] ?? [],
            "focused": attribute(element, "AXFocused") as? Bool ?? false,
        ]
        for (attributeName, resultName) in [("AXValue", "value"), ("AXMinValue", "min"), ("AXMaxValue", "max")] {
            if let scalar = attribute(element, attributeName) as? NSNumber {
                row[resultName] = scalar.doubleValue
            }
        }
        var settable = DarwinBoolean(false)
        if AXUIElementIsAttributeSettable(element, "AXValue" as CFString, &settable) == .success {
            row["value_settable"] = settable.boolValue
        }
        nodes.append(row)
    }
    for child in elements(element, "AXChildren") {
        visit(child, depth: depth + 1)
    }
}

for window in windows {
    visit(window, depth: 0)
}
var result: [String: Any] = ["trusted": true, "windows": windows.count, "progress_indicators": nodes]
if let scrollStatus { result["scroll_status"] = scrollStatus }
try report(result)
