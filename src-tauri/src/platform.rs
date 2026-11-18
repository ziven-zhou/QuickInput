// Platform-specific helpers for app focus switching and paste simulation.

pub fn get_frontmost_pid() -> i32 {
    #[cfg(target_os = "macos")]
    {
        get_frontmost_pid_macos()
    }
    #[cfg(target_os = "windows")]
    {
        get_frontmost_pid_windows()
    }
    #[cfg(target_os = "linux")]
    {
        get_frontmost_pid_linux()
    }
}

pub fn activate_pid(pid: i32) {
    if pid <= 0 {
        return;
    }
    #[cfg(target_os = "macos")]
    activate_pid_macos(pid);
    #[cfg(target_os = "windows")]
    activate_pid_windows(pid);
    #[cfg(target_os = "linux")]
    activate_pid_linux(pid);
}

pub fn can_simulate_paste() -> bool {
    #[cfg(target_os = "macos")]
    {
        accessibility_trusted_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

pub fn simulate_paste() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    return simulate_paste_macos();
    #[cfg(target_os = "windows")]
    return simulate_paste_windows();
    #[cfg(target_os = "linux")]
    return simulate_paste_linux();
}

pub(crate) struct SystemInput;

impl crate::confirmation::InputAutomation for SystemInput {
    fn frontmost_pid(&self) -> i32 {
        get_frontmost_pid()
    }
    fn keys_released(&self) -> bool {
        confirmation_keys_released()
    }
    fn paste(&mut self) -> Result<(), String> {
        simulate_paste()
    }
    fn enter(&mut self) -> Result<(), String> {
        simulate_enter()
    }
    fn wait(&mut self, millis: u64) {
        std::thread::sleep(std::time::Duration::from_millis(millis));
    }
}

#[cfg(target_os = "macos")]
fn confirmation_keys_released() -> bool {
    use core_graphics::event::CGEventFlags;
    use core_graphics::event_source::CGEventSourceStateID;
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGEventSourceKeyState(state: CGEventSourceStateID, key: u16) -> bool;
        fn CGEventSourceFlagsState(state: CGEventSourceStateID) -> u64;
    }
    // Physical HID state: do not synthesize key-up for keys the user is still holding.
    unsafe {
        let state = CGEventSourceStateID::HIDSystemState;
        let modifiers = CGEventFlags::CGEventFlagShift
            | CGEventFlags::CGEventFlagControl
            | CGEventFlags::CGEventFlagAlternate
            | CGEventFlags::CGEventFlagCommand;
        CGEventSourceFlagsState(state) & modifiers.bits() == 0
            && !CGEventSourceKeyState(state, 0x24)
            && !CGEventSourceKeyState(state, 0x4c)
    }
}

#[cfg(target_os = "windows")]
fn confirmation_keys_released() -> bool {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, VK_CONTROL, VK_LWIN, VK_MENU, VK_RETURN, VK_RWIN, VK_SHIFT,
    };
    [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN, VK_RETURN]
        .iter()
        .all(|key| unsafe { GetAsyncKeyState(key.0 as i32) >= 0 })
}

#[cfg(target_os = "linux")]
fn confirmation_keys_released() -> bool {
    use x11rb::{connection::Connection, protocol::xproto::ConnectionExt};
    let query = || -> Option<bool> {
        let (connection, _) = x11rb::connect(None).ok()?;
        let setup = connection.setup();
        let count = setup
            .max_keycode
            .checked_sub(setup.min_keycode)?
            .checked_add(1)?;
        let mapping = connection
            .get_keyboard_mapping(setup.min_keycode, count)
            .ok()?
            .reply()
            .ok()?;
        let keys = connection.query_keymap().ok()?.reply().ok()?.keys;
        let per_keycode = mapping.keysyms_per_keycode as usize;
        if per_keycode == 0 {
            return None;
        }
        for (index, symbols) in mapping.keysyms.chunks(per_keycode).enumerate() {
            // Return, keypad Enter, Shift/Control/Meta/Alt/Super/Hyper, ISO_Level3_Shift.
            if symbols
                .iter()
                .any(|symbol| matches!(*symbol, 0xff0d | 0xff8d | 0xffe1..=0xffee | 0xfe03))
            {
                let code = setup.min_keycode as usize + index;
                if keys[code / 8] & (1 << (code % 8)) != 0 {
                    return Some(false);
                }
            }
        }
        Some(true)
    };
    query().unwrap_or(false)
}

pub fn simulate_enter() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
        use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
        let source = CGEventSource::new(CGEventSourceStateID::Private)
            .map_err(|_| "无法创建回车事件源".to_string())?;
        // Create both events before posting either, and never inherit Shift/Command.
        let down = CGEvent::new_keyboard_event(source.clone(), 0x24, true)
            .map_err(|_| "无法创建回车按下事件".to_string())?;
        let up = CGEvent::new_keyboard_event(source, 0x24, false)
            .map_err(|_| "无法创建回车松开事件".to_string())?;
        down.set_flags(CGEventFlags::empty());
        up.set_flags(CGEventFlags::empty());
        down.post(CGEventTapLocation::HID);
        std::thread::sleep(std::time::Duration::from_millis(20));
        up.post(CGEventTapLocation::HID);
        Ok(())
    }
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    {
        use enigo::{Direction, Enigo, Key, Keyboard, Settings};
        let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
        enigo
            .key(Key::Return, Direction::Click)
            .map_err(|e| e.to_string())
    }
}

#[cfg(target_os = "macos")]
fn get_frontmost_pid_macos() -> i32 {
    use objc::{class, msg_send, runtime::Object, sel, sel_impl};
    unsafe {
        let workspace: *mut Object = msg_send![class!(NSWorkspace), sharedWorkspace];
        let app: *mut Object = msg_send![workspace, frontmostApplication];
        if app.is_null() {
            return 0;
        }
        msg_send![app, processIdentifier]
    }
}

#[cfg(target_os = "macos")]
fn activate_pid_macos(pid: i32) {
    use objc::{class, msg_send, runtime::Object, sel, sel_impl};
    unsafe {
        let app: *mut Object = msg_send![
            class!(NSRunningApplication),
            runningApplicationWithProcessIdentifier: pid
        ];
        if !app.is_null() {
            // NSApplicationActivateAllWindows | NSApplicationActivateIgnoringOtherApps
            let _: i32 = msg_send![app, activateWithOptions: 3];
        }
    }
}

#[cfg(target_os = "macos")]
fn accessibility_trusted_macos() -> bool {
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXIsProcessTrusted() -> u8;
    }

    unsafe { AXIsProcessTrusted() != 0 }
}

#[cfg(target_os = "macos")]
fn simulate_paste_macos() -> Result<(), String> {
    use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
    use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};
    use std::thread;
    use std::time::Duration;

    let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState)
        .map_err(|_| "failed to create keyboard event source".to_string())?;

    let command_down = CGEvent::new_keyboard_event(source.clone(), 0x37, true)
        .map_err(|_| "failed to create command key down event".to_string())?;
    command_down.set_flags(CGEventFlags::CGEventFlagCommand);
    command_down.post(CGEventTapLocation::HID);

    thread::sleep(Duration::from_millis(20));

    let v_down = CGEvent::new_keyboard_event(source.clone(), 0x09, true)
        .map_err(|_| "failed to create v key down event".to_string())?;
    v_down.set_flags(CGEventFlags::CGEventFlagCommand);
    v_down.post(CGEventTapLocation::HID);

    thread::sleep(Duration::from_millis(40));

    let v_up = CGEvent::new_keyboard_event(source.clone(), 0x09, false)
        .map_err(|_| "failed to create v key up event".to_string())?;
    v_up.set_flags(CGEventFlags::CGEventFlagCommand);
    v_up.post(CGEventTapLocation::HID);

    thread::sleep(Duration::from_millis(20));

    let command_up = CGEvent::new_keyboard_event(source, 0x37, false)
        .map_err(|_| "failed to create command key up event".to_string())?;
    command_up.post(CGEventTapLocation::HID);

    Ok(())
}

#[cfg(target_os = "windows")]
fn get_frontmost_pid_windows() -> i32 {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_invalid() {
            return 0;
        }
        let mut pid: u32 = 0;
        let _ = GetWindowThreadProcessId(hwnd, Some(&mut pid));
        pid as i32
    }
}

#[cfg(target_os = "windows")]
static mut FOUND_WINDOW: isize = 0;

#[cfg(target_os = "windows")]
unsafe extern "system" fn find_window_by_pid(hwnd: isize, lparam: isize) -> i32 {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowThreadProcessId, IsWindowVisible};
    let target = lparam as u32;
    let mut window_pid: u32 = 0;
    let hwnd = windows::Win32::Foundation::HWND(hwnd as *mut _);
    let _ = GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
    if window_pid == target && IsWindowVisible(hwnd).as_bool() {
        FOUND_WINDOW = hwnd.0 as isize;
        0
    } else {
        1
    }
}

#[cfg(target_os = "windows")]
fn activate_pid_windows(pid: i32) {
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    unsafe {
        FOUND_WINDOW = 0;
        let _ = EnumWindows(Some(find_window_by_pid), LPARAM(pid as isize));
        if FOUND_WINDOW != 0 {
            let hwnd = HWND(FOUND_WINDOW as *mut _);
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}
#[cfg(target_os = "windows")]
static FOUND_WINDOW: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

#[cfg(target_os = "windows")]
unsafe extern "system" fn find_window_by_pid(hwnd: isize, lparam: isize) -> i32 {
    use std::sync::atomic::Ordering;
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowThreadProcessId, IsWindowVisible};
    let target = lparam as u32;
    let mut window_pid: u32 = 0;
    let hwnd = windows::Win32::Foundation::HWND(hwnd as *mut _);
    let _ = GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
    if window_pid == target && IsWindowVisible(hwnd).as_bool() {
        FOUND_WINDOW.store(hwnd.0 as isize, Ordering::SeqCst);
        0
    } else {
        1
    }
}

#[cfg(target_os = "windows")]
fn activate_pid_windows(pid: i32) {
    use std::sync::atomic::Ordering;
    use windows::Win32::Foundation::{HWND, LPARAM};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    unsafe {
        FOUND_WINDOW.store(0, Ordering::SeqCst);
        let _ = EnumWindows(Some(find_window_by_pid), LPARAM(pid as isize));
        let handle = FOUND_WINDOW.load(Ordering::SeqCst);
        if handle != 0 {
            let hwnd = HWND(handle as *mut _);
            let _ = ShowWindow(hwnd, SW_RESTORE);
            let _ = SetForegroundWindow(hwnd);
        }
    }
}

#[cfg(target_os = "windows")]
fn simulate_paste_windows() -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn get_frontmost_pid_linux() -> i32 {
    if let Ok(out) = std::process::Command::new("xdotool")
        .args(["getactivewindow", "getwindowpid"])
        .output()
    {
        if let Ok(pid_str) = String::from_utf8(out.stdout) {
            return pid_str.trim().parse().unwrap_or(0);
        }
    }
    0
}

#[cfg(target_os = "linux")]
fn activate_pid_linux(pid: i32) {
    if let Ok(out) = std::process::Command::new("xdotool")
        .args(["search", "--pid", &pid.to_string()])
        .output()
    {
        if let Ok(windows) = String::from_utf8(out.stdout) {
            if let Some(window_id) = windows.lines().next() {
                let _ = std::process::Command::new("xdotool")
                    .args(["windowactivate", window_id])
                    .spawn();
            }
        }
    }
}

#[cfg(target_os = "linux")]
fn simulate_paste_linux() -> Result<(), String> {
    use enigo::{Direction, Enigo, Key, Keyboard, Settings};
    let mut enigo = Enigo::new(&Settings::default()).map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Press)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Unicode('v'), Direction::Click)
        .map_err(|e| e.to_string())?;
    enigo
        .key(Key::Control, Direction::Release)
        .map_err(|e| e.to_string())?;
    Ok(())
}
