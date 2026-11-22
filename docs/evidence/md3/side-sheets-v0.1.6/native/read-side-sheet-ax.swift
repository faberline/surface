import ApplicationServices
import AppKit
import Foundation

func attribute(_ node: AXUIElement, _ name: String) -> Any? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(node, name as CFString, &value) == .success ? value : nil
}

func elements(_ node: AXUIElement, _ name: String) -> [AXUIElement] {
    attribute(node, name) as? [AXUIElement] ?? []
}

func element(_ node: AXUIElement, _ name: String) -> AXUIElement? {
    guard let value = attribute(node, name) as CFTypeRef?,
          CFGetTypeID(value) == AXUIElementGetTypeID() else { return nil }
    return (value as! AXUIElement)
}

func report(_ value: [String: Any]) throws {
    let encoded = try JSONSerialization.data(withJSONObject: value, options: [.sortedKeys])
    FileHandle.standardOutput.write(encoded + Data([0x0A]))
}

let args = CommandLine.arguments
guard (2...3).contains(args.count),
      let pid = pid_t(args[1]), pid > 1 else {
    fatalError("Usage: read-side-sheet-ax PID [close|scroll]")
}
guard AXIsProcessTrusted() else {
    try report(["trusted": false])
    exit(0)
}
let command = args.count == 3 ? args[2] : ""
let gallery = NSRunningApplication(processIdentifier: pid)
if command == "focus", gallery?.isActive == false {
    _ = gallery?.activate(options: [.activateAllWindows])
}
let active = gallery?.isActive ?? false
let app = AXUIElementCreateApplication(pid)
AXUIElementSetMessagingTimeout(app, 0.5)
let focus = element(app, "AXFocusedUIElement")
var windowsValue: CFTypeRef?
let windowsStatus = AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &windowsValue)
let windows = windowsValue as? [AXUIElement] ?? []
let roots: [AXUIElement]
let rootSource: String
if !windows.isEmpty {
    roots = windows
    rootSource = "AXWindows"
} else if let focusedWindow = element(app, "AXFocusedWindow") {
    roots = [focusedWindow]
    rootSource = "AXFocusedWindow"
} else if let mainWindow = element(app, "AXMainWindow") {
    roots = [mainWindow]
    rootSource = "AXMainWindow"
} else if var ancestor = focus {
    for _ in 0..<16 {
        guard let parent = element(ancestor, "AXParent"),
              !CFEqual(parent, app), !CFEqual(parent, ancestor) else { break }
        ancestor = parent
    }
    roots = [ancestor]
    rootSource = "AXFocusedUIElement"
} else {
    roots = []
    rootSource = "unavailable"
}
let names: Set<String> = [
    "工作區詳細資料", "更新頁面", "儲存設定", "解除封存", "更新紀錄", "關閉側邊面板",
]
var found: [[String: Any]] = []
var actionStatus: Int32?
var visits = 0

func visit(_ element: AXUIElement, depth: Int) {
    visits += 1
    guard depth < 32 && visits < 1800 else { return }
    let role = attribute(element, "AXRole") as? String ?? ""
    let subrole = attribute(element, "AXSubrole") as? String ?? ""
    let candidates = ["AXTitle", "AXLabel", "AXDescription"]
        .compactMap { attribute(element, $0) as? String }
    let name = candidates.first(where: { names.contains($0) }) ?? ""
    var exportedActions: CFArray?
    AXUIElementCopyActionNames(element, &exportedActions)
    let actions = exportedActions as? [String] ?? []
    if !name.isEmpty || subrole == "AXDialog" {
        found.append([
            "name": name, "role": role, "subrole": subrole,
            "enabled": attribute(element, "AXEnabled") as? Bool ?? false,
            "focused": attribute(element, "AXFocused") as? Bool ?? false,
            "actions": actions,
        ])
    }
    // The native host accepts accessibility actions only while its own
    // window has system focus. Leave inactive windows available for reading.
    if active && actionStatus == nil {
        if command == "close" && name == "關閉側邊面板" && actions.contains("AXPress") {
            actionStatus = AXUIElementPerformAction(element, kAXPressAction as CFString).rawValue
        } else if command == "scroll" && actions.contains("AXScrollDownByPage") {
            actionStatus = AXUIElementPerformAction(element, "AXScrollDownByPage" as CFString).rawValue
        }
    }
    for child in elements(element, "AXChildren") {
        visit(child, depth: depth + 1)
    }
}

for window in roots {
    visit(window, depth: 0)
}
var result: [String: Any] = [
    "trusted": true, "windows": windows.count, "windows_status": windowsStatus.rawValue,
    "active": active, "root_source": rootSource, "objects": found,
]
if let actionStatus { result["action_status"] = actionStatus }
try report(result)
