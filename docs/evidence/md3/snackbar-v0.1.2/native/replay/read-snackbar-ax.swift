import ApplicationServices
import Foundation

func attribute(_ element: AXUIElement, _ key: String) -> Any? {
    var value: CFTypeRef?
    return AXUIElementCopyAttributeValue(element, key as CFString, &value) == .success ? value : nil
}

func children(_ element: AXUIElement, _ key: String) -> [AXUIElement] {
    attribute(element, key) as? [AXUIElement] ?? []
}

func name(_ element: AXUIElement) -> String {
    for attributeName in ["AXTitle", "AXDescription", "AXValue", "AXLabel"] {
        if let value = attribute(element, attributeName) as? String, !value.isEmpty {
            return value
        }
    }
    return ""
}

func report(_ value: [String: Any]) {
    guard let encoded = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]) else {
        return
    }
    FileHandle.standardOutput.write(encoded + Data([0x0A]))
}

let args = CommandLine.arguments
guard args.count >= 3, let pid = pid_t(args[1]), pid > 1 else {
    fatalError("Usage: read-snackbar-ax PID --read | --observe SECONDS")
}
let trusted = AXIsProcessTrusted()
guard trusted else {
    report(["trusted": false, "registered": false])
    exit(0)
}

let app = AXUIElementCreateApplication(pid)
AXUIElementSetMessagingTimeout(app, 0.5)
_ = attribute(app, "AXFocusedUIElement")
let windows = children(app, "AXWindows")

if args[2] == "--read" || args[2] == "--read-status" {
    let statusOnly = args[2] == "--read-status"
    var buttons: [[String: Any]] = []
    var statuses: [[String: Any]] = []
    var visits = 0
    func walk(_ element: AXUIElement, depth: Int) {
        visits += 1
        guard depth < 32 && visits < 1400 && (!statusOnly || statuses.isEmpty) else { return }
        let role = attribute(element, "AXRole") as? String ?? ""
        let subrole = attribute(element, "AXSubrole") as? String ?? ""
        if !statusOnly && role == "AXButton" {
            var actions: CFArray?
            AXUIElementCopyActionNames(element, &actions)
            buttons.append([
                "name": name(element),
                "enabled": attribute(element, "AXEnabled") as? Bool ?? false,
                "focused": attribute(element, "AXFocused") as? Bool ?? false,
                "actions": actions as? [String] ?? [],
            ])
        }
        if subrole == "AXApplicationStatus" {
            statuses.append([
                "label": name(element),
                "role": role,
                "subrole": subrole,
                "live_status": attribute(element, "AXLiveRegionStatus") as? String ?? "",
            ])
        }
        let descendants = children(element, "AXChildren")
        let order = statusOnly ? Array(descendants.reversed()) : descendants
        for child in order {
            walk(child, depth: depth + 1)
        }
    }
    for window in windows {
        walk(window, depth: 0)
    }
    report(["trusted": true, "windows": windows.count, "statuses": statuses, "buttons": buttons])
} else if args[2] == "--observe", args.count == 4, let seconds = Double(args[3]),
          (1...15).contains(seconds) {
    let notification = "AXAnnouncementRequested" as CFString
    var observer: AXObserver?
    let created = AXObserverCreateWithInfoCallback(
        pid,
        { _, _, name, info, _ in
            let fields = info as? [String: Any] ?? [:]
            let key = fields.keys.first { $0.lowercased().contains("announcement") }
            let text = key.flatMap { fields[$0] as? String } ?? ""
            let priorityKey = fields.keys.first { $0.lowercased().contains("priority") }
            let priority = priorityKey.flatMap { fields[$0] as? Int } ?? -1
            report(["notification": name as String, "announcement": text, "priority": priority])
        },
        &observer
    )
    guard created == .success, let observer else {
        report(["trusted": true, "registered": false, "create_code": created.rawValue])
        exit(0)
    }

    // AccessKit posts the notification on the native window. Keep AppKit's
    // registration result when remote observers cannot subscribe.
    let registration = windows.map { AXObserverAddNotification(observer, $0, notification, nil) }
    let accepted = registration.contains(.success)
    report([
        "trusted": true,
        "registered": accepted,
        "windows": windows.count,
        "codes": registration.map { $0.rawValue },
    ])
    if accepted {
        CFRunLoopAddSource(CFRunLoopGetCurrent(), AXObserverGetRunLoopSource(observer), .defaultMode)
        CFRunLoopRunInMode(.defaultMode, seconds, false)
    }
} else {
    fatalError("Invalid AppKit probe option")
}
