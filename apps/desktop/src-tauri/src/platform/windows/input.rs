use std::{env, mem, thread, time::Duration};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, MapVirtualKeyW, SendInput, VkKeyScanW, INPUT, INPUT_0, INPUT_KEYBOARD,
    INPUT_MOUSE, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_SCANCODE, KEYEVENTF_UNICODE,
    MAP_VIRTUAL_KEY_TYPE,
    MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP, MOUSEINPUT, VIRTUAL_KEY, VK_CONTROL,
    VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RMENU, VK_RSHIFT,
    VK_RWIN, VK_SHIFT, VK_V,
};
use windows::Win32::UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow};

pub(crate) fn paste_text_into_focused_field(
    text: &str,
    keybind: Option<&str>,
) -> Result<(), String> {
    if text.trim().is_empty() {
        return Ok(());
    }

    let override_text = env::var("VOQUILL_DEBUG_PASTE_TEXT").ok();
    let target = override_text.as_deref().unwrap_or(text);
    log::info!(
        "attempting to inject text ({} chars)",
        target.chars().count()
    );

    paste_via_clipboard(target, keybind).or_else(|err| {
        log::warn!("Clipboard paste failed ({err}), falling back to simulated typing");
        use enigo::{Enigo, KeyboardControllable};
        let mut enigo = Enigo::new();
        release_modifier_keys();
        thread::sleep(Duration::from_millis(50));
        enigo.key_sequence(target);
        Ok(())
    })
}

fn is_console_window() -> bool {
    unsafe {
        let hwnd: HWND = GetForegroundWindow();
        if hwnd.0.is_null() {
            return false;
        }

        let mut class_name = [0u16; 256];
        let len = GetClassNameW(hwnd, &mut class_name);
        if len == 0 {
            return false;
        }

        let class_str = String::from_utf16_lossy(&class_name[..len as usize]);
        log::debug!("foreground window class: {}", class_str);

        class_str == "ConsoleWindowClass"
    }
}

fn release_modifier_keys() {
    let win_held = is_key_pressed(VK_LWIN) || is_key_pressed(VK_RWIN);
    if win_held {
        cancel_pending_start_menu();
    }

    let modifiers = [
        VK_SHIFT,
        VK_CONTROL,
        VK_MENU,
        VK_LSHIFT,
        VK_RSHIFT,
        VK_LCONTROL,
        VK_RCONTROL,
        VK_LMENU,
        VK_RMENU,
        VK_LWIN,
        VK_RWIN,
    ];

    for vk in modifiers {
        if is_key_pressed(vk) {
            send_key_up(vk);
        }
    }
}

fn cancel_pending_start_menu() {
    let down = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0xFF),
                wScan: 0,
                dwFlags: Default::default(),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let up = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: VIRTUAL_KEY(0xFF),
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        SendInput(&[down, up], mem::size_of::<INPUT>() as i32);
    }
}

fn is_key_pressed(vk: VIRTUAL_KEY) -> bool {
    unsafe { GetAsyncKeyState(vk.0 as i32) < 0 }
}

fn send_key_down(vk: VIRTUAL_KEY) {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: Default::default(),
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        SendInput(&[input], mem::size_of::<INPUT>() as i32);
    }
}

fn send_key_up(vk: VIRTUAL_KEY) {
    let input = INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: KEYEVENTF_KEYUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        SendInput(&[input], mem::size_of::<INPUT>() as i32);
    }
}

fn send_right_click() {
    let down = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_RIGHTDOWN,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    let up = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: MOUSEEVENTF_RIGHTUP,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };
    unsafe {
        SendInput(&[down], mem::size_of::<INPUT>() as i32);
        thread::sleep(Duration::from_millis(10));
        SendInput(&[up], mem::size_of::<INPUT>() as i32);
    }
}

fn send_paste_keys(keybind: Option<&str>) {
    let is_console = is_console_window();

    let use_shift = keybind == Some("ctrl+shift+v");
    if is_console && !use_shift {
        log::info!("detected console window, using right-click to paste");
        send_right_click();
    } else {
        send_key_down(VK_CONTROL);
        if use_shift {
            send_key_down(VK_SHIFT);
        }
        send_key_down(VK_V);
        thread::sleep(Duration::from_millis(20));
        send_key_up(VK_V);
        if use_shift {
            send_key_up(VK_SHIFT);
        }
        send_key_up(VK_CONTROL);
    }
}

pub(crate) fn send_unicode_string(text: &str) {
    release_modifier_keys();
    thread::sleep(Duration::from_millis(30));

    for c in text.chars() {
        unsafe {
            let vk_result = VkKeyScanW(c as u16);
            if vk_result == -1 {
                // No VK mapping (emoji, CJK, etc.) — fall back to KEYEVENTF_UNICODE
                let code = c as u16;
                let down = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: code,
                            dwFlags: KEYEVENTF_UNICODE,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                let up = INPUT {
                    r#type: INPUT_KEYBOARD,
                    Anonymous: INPUT_0 {
                        ki: KEYBDINPUT {
                            wVk: VIRTUAL_KEY(0),
                            wScan: code,
                            dwFlags: KEYEVENTF_UNICODE | KEYEVENTF_KEYUP,
                            time: 0,
                            dwExtraInfo: 0,
                        },
                    },
                };
                SendInput(&[down, up], mem::size_of::<INPUT>() as i32);
                thread::sleep(Duration::from_millis(10));
                continue;
            }
            let vk = VIRTUAL_KEY((vk_result & 0xFF) as u16);
            let modifiers = (vk_result >> 8) & 0xFF;
            let need_shift = modifiers & 1 != 0;
            let need_ctrl = modifiers & 2 != 0;
            let need_alt = modifiers & 4 != 0;
            let scan = MapVirtualKeyW(vk.0 as u32, MAP_VIRTUAL_KEY_TYPE(0)) as u16;

            // Press modifiers
            if need_shift {
                send_key_down(VK_SHIFT);
            }
            if need_ctrl {
                send_key_down(VK_CONTROL);
            }
            if need_alt {
                send_key_down(VK_MENU);
            }

            // Press and release the key using scan codes
            let down = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: vk,
                        wScan: scan,
                        dwFlags: KEYEVENTF_SCANCODE,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            let up = INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 {
                    ki: KEYBDINPUT {
                        wVk: vk,
                        wScan: scan,
                        dwFlags: KEYEVENTF_SCANCODE | KEYEVENTF_KEYUP,
                        time: 0,
                        dwExtraInfo: 0,
                    },
                },
            };
            SendInput(&[down, up], mem::size_of::<INPUT>() as i32);

            // Release modifiers
            if need_alt {
                send_key_up(VK_MENU);
            }
            if need_ctrl {
                send_key_up(VK_CONTROL);
            }
            if need_shift {
                send_key_up(VK_SHIFT);
            }
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn paste_via_clipboard(text: &str, keybind: Option<&str>) -> Result<(), String> {
    let mut clipboard =
        arboard::Clipboard::new().map_err(|err| format!("clipboard unavailable: {err}"))?;
    let previous = clipboard.get_text().ok();
    clipboard
        .set_text(text.to_string())
        .map_err(|err| format!("failed to store clipboard text: {err}"))?;

    thread::sleep(Duration::from_millis(50));

    release_modifier_keys();
    thread::sleep(Duration::from_millis(30));

    send_paste_keys(keybind);

    if let Some(old) = previous {
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(800));
            if let Ok(mut clipboard) = arboard::Clipboard::new() {
                let _ = clipboard.set_text(old);
            }
        });
    }

    Ok(())
}
