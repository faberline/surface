import ApplicationServices
import Foundation

func attribute(_ element: AXUIElement, _ key: String) -> Any? {
    var result: CFTypeRef?
    guard AXUIElementCopyAttributeValue(element, key as CFString, &result) == .success else { return nil }
    return result
}

func scan(_ element: AXUIElement, _ found: inout [AXUIElement], _ depth: Int = 0) {
    guard depth < 18 else { return }
    if attribute(element, "AXRole") as? String == "AXSlider" { found.append(element) }
    for child in attribute(element, "AXChildren") as? [AXUIElement] ?? [] {
        scan(child, &found, depth + 1)
    }
}

func printJSON(_ result: [String: Any]) throws {
    print(String(decoding: try JSONSerialization.data(withJSONObject: result, options: [.sortedKeys]), as: UTF8.self))
}

let args = CommandLine.arguments
guard args.count == 3, let pid = Int32(args[1]), let requested = Double(args[2]) else { exit(64) }
guard AXIsProcessTrusted() else {
    try printJSON(["trusted": false])
    exit(77)
}
let app = AXUIElementCreateApplication(pid)
AXUIElementSetMessagingTimeout(app, 2)
_ = attribute(app, "AXFocusedUIElement")
var sliders: [AXUIElement] = []
let windows = attribute(app, "AXWindows") as? [AXUIElement] ?? []
for window in windows {
    scan(window, &sliders)
}
guard sliders.count == 1, let slider = sliders.first else {
    try printJSON(["trusted": true, "windows": windows.count, "sliders_found": sliders.count])
    exit(65)
}
let value = "AXValue" as CFString
let before = attribute(slider, "AXValue") as? NSNumber
let minimum = attribute(slider, "AXMinValue") as? NSNumber
let maximum = attribute(slider, "AXMaxValue") as? NSNumber
var settable = DarwinBoolean(false)
let isSettable = AXUIElementIsAttributeSettable(slider, value, &settable)
var actions: CFArray?
AXUIElementCopyActionNames(slider, &actions)
var written: Int32?
if isSettable == .success && settable.boolValue {
    written = AXUIElementSetAttributeValue(slider, value, NSNumber(value: requested)).rawValue
}
Thread.sleep(forTimeInterval: 0.2)
let after = attribute(slider, "AXValue") as? NSNumber
let labels = ["AXTitle", "AXDescription", "AXLabel", "AXHelp"]
    .compactMap { attribute(slider, $0) as? String }
var info: [String: Any] = [
    "trusted": true, "role": "AXSlider", "name": labels.first { !$0.isEmpty } ?? "",
    "requested": requested, "is_attribute_settable_status": isSettable.rawValue,
    "value_settable": settable.boolValue, "actions": actions as? [String] ?? [],
]
if let before { info["before"] = before }
if let after { info["after"] = after }
if let minimum { info["min"] = minimum }
if let maximum { info["max"] = maximum }
if let written { info["set_attribute_status"] = written }
try printJSON(info)
