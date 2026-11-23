import ctypes

from gallery_transport import require


def onscreen_windows(pid):
    """Ask CoreGraphics about the live child only. Do not capture the desktop."""
    cg = ctypes.CDLL("/System/Library/Frameworks/CoreGraphics.framework/CoreGraphics")
    cf = ctypes.CDLL("/System/Library/Frameworks/CoreFoundation.framework/CoreFoundation")
    pointer = ctypes.c_void_p
    cg.CGWindowListCopyWindowInfo.argtypes = [ctypes.c_uint32, ctypes.c_uint32]
    cg.CGWindowListCopyWindowInfo.restype = pointer
    cf.CFArrayGetCount.argtypes = [pointer]
    cf.CFArrayGetCount.restype = ctypes.c_long
    cf.CFArrayGetValueAtIndex.argtypes = [pointer, ctypes.c_long]
    cf.CFArrayGetValueAtIndex.restype = pointer
    cf.CFDictionaryGetValue.argtypes = [pointer, pointer]
    cf.CFDictionaryGetValue.restype = pointer
    cf.CFNumberGetValue.argtypes = [pointer, ctypes.c_long, pointer]
    cf.CFNumberGetValue.restype = ctypes.c_bool
    cf.CFRelease.argtypes = [pointer]
    owner_key = pointer.in_dll(cg, "kCGWindowOwnerPID")
    windows = cg.CGWindowListCopyWindowInfo(1, 0)
    require(windows, "CoreGraphics did not return a window list")
    try:
        count = 0
        for index in range(cf.CFArrayGetCount(windows)):
            entry = cf.CFArrayGetValueAtIndex(windows, index)
            owner = cf.CFDictionaryGetValue(entry, owner_key)
            value = ctypes.c_int32()
            if owner and cf.CFNumberGetValue(owner, 3, ctypes.byref(value)) and value.value == pid:
                count += 1
        return count
    finally:
        cf.CFRelease(windows)
