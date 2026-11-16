//! Event-driven Windows adapter. No Raw Input, hooks, animation worker, audio or GPU loop.
mod paint;

use crate::{
    model::{self, State, Timer},
    storage,
};
use ::windows::{
    Win32::{
        Foundation::*,
        Globalization::{ENUM_DATE_FORMATS_FLAGS, GetDateFormatEx},
        Graphics::Gdi::*,
        System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
        UI::{HiDpi::*, Input::KeyboardAndMouse::*, Shell::*, WindowsAndMessaging::*},
    },
    core::{Error, PCWSTR, w},
};
use std::{cell::RefCell, path::PathBuf, time::Instant};
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
const TRAY_MESSAGE: u32 = WM_APP + 1;
const HOTKEY: i32 = 1;
// WM_MOUSELEAVE from the Windows SDK; no common-controls library is needed.
const WM_MOUSELEAVE: u32 = 0x02a3;

struct Drag {
    origin: POINT,
    scale: u16,
    moved: bool,
}
struct App {
    state: State,
    saved: State,
    timer: Timer,
    path: PathBuf,
    started: Instant,
    avatar: image::RgbaImage,
    pet: HWND,
    panel: HWND,
    dpi: u32,
    wheel: i32,
    drag: Option<Drag>,
    hover: bool,
    panel_running: bool,
    panel_key: (u64, u64, u64),
    last_saved: u64,
    save_failed: bool,
    smoke_seconds: Option<u64>,
}
impl App {
    fn now(&self) -> u64 {
        self.started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
    }
    fn persist(&mut self) -> Result<()> {
        self.state.total_millis = self.timer.total(self.now());
        self.state.update_day(today()?);
        if self.state != self.saved {
            storage::save(&self.path, &self.state)?;
            self.saved = self.state.clone();
        }
        self.save_failed = false;
        Ok(())
    }
    fn report_error(&mut self) {
        if self.save_failed {
            return;
        }
        self.save_failed = true;
        // SAFETY: tray data contains only owned, bounded text and this UI thread's HWND.
        unsafe {
            let mut data = NOTIFYICONDATAW {
                cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                hWnd: self.pet,
                uID: 1,
                uFlags: NIF_INFO,
                dwInfoFlags: NIIF_ERROR,
                ..Default::default()
            };
            wide_copy(&mut data.szInfoTitle, "BongoCat 工作计时");
            wide_copy(
                &mut data.szInfo,
                "无法保存工作时间或更新桌宠窗口。请检查数据目录写入权限，然后重试。",
            );
            let _ = Shell_NotifyIconW(NIM_MODIFY, &data);
        }
    }
    fn pet_rect(&self) -> Result<RECT> {
        let mut rect = RECT::default();
        // SAFETY: live owned window, writable local result.
        unsafe {
            GetWindowRect(self.pet, &mut rect)?;
        }
        Ok(rect)
    }
    fn redraw_pet(&mut self) -> Result<()> {
        let width = (350 * u32::from(self.state.scale) * self.dpi / 9600).clamp(60, 5600);
        let image = paint::avatar(&self.avatar, width);
        let (x, y) = self.state.position.unwrap_or((100, 100));
        image.present(
            self.pet,
            POINT { x, y },
            (u16::from(self.state.opacity) * 255 / 100) as u8,
        )?;
        Ok(())
    }
    fn hide_panel(&mut self) {
        // SAFETY: window is owned by this UI thread.
        unsafe {
            let _ = ShowWindow(self.panel, SW_HIDE);
        }
        self.hover = false;
    }
    fn panel_visible(&self) -> bool {
        unsafe { IsWindowVisible(self.panel).as_bool() }
    }
    fn redraw_panel(&mut self) -> Result<()> {
        let now = self.now();
        let total = self.timer.total(now);
        let elapsed = self.timer.elapsed(now);
        let image = paint::panel(
            self.state.days,
            total,
            elapsed,
            self.timer.running(),
            self.hover,
            self.dpi,
        )?;
        let anchor = self.pet_rect()?;
        let work = monitor_work(anchor)?;
        let x = (anchor.left + (anchor.right - anchor.left - image.width) / 2)
            .clamp(work.left, (work.right - image.width).max(work.left));
        let y = (anchor.top - image.height - 6)
            .clamp(work.top, (work.bottom - image.height).max(work.top));
        image.present(self.panel, POINT { x, y }, 255)?;
        self.panel_running = self.timer.running();
        self.panel_key = (self.state.days, total / 180_000, elapsed / 1000);
        Ok(())
    }
    fn toggle_panel(&mut self) -> Result<()> {
        if self.panel_visible() {
            self.hide_panel();
            return Ok(());
        }
        self.redraw_panel()?;
        // SAFETY: show the owned popup without changing its owner or storage lifetime.
        unsafe {
            let _ = SetWindowPos(
                self.panel,
                Some(HWND_TOPMOST),
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE,
            );
            let _ = ShowWindow(self.panel, SW_SHOW);
            let _ = SetForegroundWindow(self.panel);
        }
        Ok(())
    }
    fn toggle_timer(&mut self) -> Result<()> {
        self.timer.toggle(self.now());
        if self.panel_visible() {
            self.redraw_panel()?;
        }
        self.update_tray()?;
        self.persist()
    }
    fn update_tray(&self) -> Result<()> {
        let mut data = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: self.pet,
            uID: 1,
            uFlags: NIF_TIP,
            ..Default::default()
        };
        wide_copy(
            &mut data.szTip,
            if self.timer.running() {
                "BongoCat · 专注中 · Ctrl+Alt+L 停止"
            } else {
                "BongoCat · 休息中 · Ctrl+Alt+L 开始"
            },
        );
        unsafe {
            if !Shell_NotifyIconW(NIM_MODIFY, &data).as_bool() {
                return Err(Error::from_thread().into());
            }
        }
        Ok(())
    }
    fn button_at(&self, lparam: LPARAM) -> bool {
        let x = lparam.0 as u16 as i16 as f32 * 96.0 / self.dpi as f32;
        let y = (lparam.0 >> 16) as u16 as i16 as f32 * 96.0 / self.dpi as f32;
        (30.0..298.0).contains(&x) && (134.0..173.0).contains(&y)
    }
    fn resize(&mut self, scale: u16) -> Result<()> {
        if scale == self.state.scale {
            return Ok(());
        }
        self.state.scale = scale.clamp(25, 400);
        self.hide_panel();
        self.redraw_pet()
    }
    fn quit(&mut self) -> Result<()> {
        self.timer.stop(self.now());
        self.persist()?;
        unsafe {
            PostQuitMessage(0);
        }
        Ok(())
    }
    fn menu(&mut self) -> Result<()> {
        // SAFETY: menu ownership is local, strings stay live through each synchronous append.
        unsafe {
            let menu = CreatePopupMenu()?;
            let result = (|| -> Result<()> {
                let items = [
                    (
                        1,
                        if self.timer.running() {
                            "停止工作计时\tCtrl+Alt+L"
                        } else {
                            "开始工作计时\tCtrl+Alt+L"
                        },
                    ),
                    (2, "打开时光面板"),
                    (0, ""),
                    (3, "始终置顶"),
                    (4, "缩小"),
                    (5, "放大"),
                    (6, "恢复默认大小"),
                    (7, "头像透明度 60%"),
                    (8, "头像透明度 80%"),
                    (9, "头像透明度 100%"),
                    (0, ""),
                    (10, "退出"),
                ];
                for (id, label) in items {
                    let text = wide(label);
                    let flags = if id == 0 {
                        MF_SEPARATOR
                    } else if id == 3 && self.state.always_on_top {
                        MF_STRING | MF_CHECKED
                    } else {
                        MF_STRING
                    };
                    AppendMenuW(menu, flags, id, PCWSTR(text.as_ptr()))?;
                }
                let mut point = POINT::default();
                GetCursorPos(&mut point)?;
                let _ = SetForegroundWindow(self.pet);
                let selected = TrackPopupMenu(
                    menu,
                    TPM_RETURNCMD | TPM_RIGHTBUTTON,
                    point.x,
                    point.y,
                    None,
                    self.pet,
                    None,
                )
                .0;
                match selected {
                    1 => self.toggle_timer()?,
                    2 => self.toggle_panel()?,
                    3 => {
                        self.state.always_on_top = !self.state.always_on_top;
                        SetWindowPos(
                            self.pet,
                            Some(if self.state.always_on_top {
                                HWND_TOPMOST
                            } else {
                                HWND_NOTOPMOST
                            }),
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        )?;
                    }
                    4 => self.resize(self.state.scale.saturating_sub(5))?,
                    5 => self.resize(self.state.scale.saturating_add(5))?,
                    6 => self.resize(100)?,
                    7..=9 => {
                        self.state.opacity = match selected {
                            7 => 60,
                            8 => 80,
                            _ => 100,
                        };
                        self.redraw_pet()?;
                    }
                    10 => self.quit()?,
                    _ => {}
                }
                self.persist()
            })();
            let _ = DestroyMenu(menu);
            result
        }
    }
    fn message(
        &mut self,
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> Result<Option<LRESULT>> {
        let popup = hwnd == self.panel;
        match msg {
            WM_CLOSE => {
                if popup {
                    self.hide_panel();
                } else {
                    self.quit()?;
                }
            }
            WM_QUERYENDSESSION => return Ok(Some(LRESULT(1))),
            WM_ENDSESSION if wparam.0 != 0 => {
                self.quit()?;
            }
            WM_HOTKEY if wparam.0 == HOTKEY as usize => self.toggle_timer()?,
            WM_ACTIVATE if popup && (wparam.0 & 0xffff) == WA_INACTIVE as usize => {
                if HWND(lparam.0 as *mut _) != self.pet {
                    self.hide_panel();
                }
            }
            WM_KEYDOWN if popup => match wparam.0 {
                27 => self.hide_panel(),
                13 | 32 => self.toggle_timer()?,
                _ => {}
            },
            WM_LBUTTONUP if popup => {
                if self.button_at(lparam) {
                    self.toggle_timer()?;
                }
            }
            WM_MOUSEMOVE if popup => {
                let hover = self.button_at(lparam);
                if hover != self.hover {
                    self.hover = hover;
                    self.redraw_panel()?;
                }
                let mut tracking = TRACKMOUSEEVENT {
                    cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32,
                    dwFlags: TME_LEAVE,
                    hwndTrack: hwnd,
                    dwHoverTime: 0,
                };
                unsafe {
                    let _ = TrackMouseEvent(&mut tracking);
                }
            }
            WM_MOUSELEAVE if popup => {
                if self.hover {
                    self.hover = false;
                    self.redraw_panel()?;
                }
            }
            WM_RBUTTONUP if popup => self.hide_panel(),
            WM_NCHITTEST if !popup => return Ok(Some(LRESULT(HTCAPTION as isize))),
            WM_NCLBUTTONDOWN if !popup => {
                self.hide_panel();
                return Ok(None);
            }
            WM_NCRBUTTONDOWN | WM_RBUTTONDOWN if !popup => {
                let mut origin = POINT::default();
                unsafe {
                    GetCursorPos(&mut origin)?;
                    SetCapture(self.pet);
                }
                self.drag = Some(Drag {
                    origin,
                    scale: self.state.scale,
                    moved: false,
                });
            }
            WM_NCMOUSEMOVE | WM_MOUSEMOVE if !popup && self.drag.is_some() => {
                let mut point = POINT::default();
                unsafe {
                    GetCursorPos(&mut point)?;
                }
                let drag = self.drag.as_mut().expect("drag exists");
                let dx = point.x - drag.origin.x;
                let dy = point.y - drag.origin.y;
                drag.moved |= dx.abs().max(dy.abs()) >= 3;
                if drag.moved {
                    let scale = (i32::from(drag.scale) + (dx + dy) * 96 / (2 * self.dpi as i32))
                        .clamp(25, 400) as u16;
                    self.resize(scale)?;
                }
            }
            WM_NCRBUTTONUP | WM_RBUTTONUP if !popup => {
                let drag = self.drag.take();
                unsafe {
                    let _ = ReleaseCapture();
                }
                if let Some(drag) = drag {
                    if drag.moved {
                        self.persist()?;
                    } else {
                        self.toggle_panel()?;
                    }
                }
            }
            WM_CAPTURECHANGED if !popup => {
                self.drag = None;
            }
            WM_MOUSEWHEEL if !popup && self.drag.is_none() => {
                let rect = self.pet_rect()?;
                let x = lparam.0 as u16 as i16 as i32;
                let y = (lparam.0 >> 16) as u16 as i16 as i32;
                if x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom {
                    let scale = model::wheel_scale(
                        self.state.scale,
                        &mut self.wheel,
                        (wparam.0 >> 16) as u16 as i16,
                    );
                    self.resize(scale)?;
                    self.persist()?;
                }
            }
            WM_EXITSIZEMOVE if !popup => {
                let rect = self.pet_rect()?;
                self.state.position = Some((rect.left, rect.top));
                self.hide_panel();
                self.persist()?;
            }
            WM_DPICHANGED if !popup => {
                self.dpi = (wparam.0 & 0xffff) as u32;
                self.hide_panel();
                self.redraw_pet()?;
            }
            WM_TIMER if !popup => {
                if self
                    .smoke_seconds
                    .is_some_and(|limit| self.started.elapsed().as_secs() >= limit)
                {
                    self.quit()?;
                    return Ok(Some(LRESULT(0)));
                }
                let now = self.now();
                self.state.update_day(today()?);
                if now.saturating_sub(self.last_saved) >= 5000 {
                    self.last_saved = now;
                    self.persist()?;
                }
                let key = (
                    self.state.days,
                    self.timer.total(now) / 180_000,
                    self.timer.elapsed(now) / 1000,
                );
                if self.panel_visible()
                    && (self.panel_key != key || self.panel_running != self.timer.running())
                {
                    self.redraw_panel()?;
                }
            }
            TRAY_MESSAGE => match lparam.0 as u32 {
                WM_RBUTTONUP | WM_CONTEXTMENU => self.menu()?,
                WM_LBUTTONUP => self.toggle_panel()?,
                _ => {}
            },
            _ => return Ok(None),
        }
        Ok(Some(LRESULT(0)))
    }
}

unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    // SAFETY: user32 invokes this on the creating thread. RefCell rejects reentrant mutable
    // callbacks while synchronous Win32 calls are on the stack. State outlives both HWNDs.
    unsafe {
        if msg == WM_NCCREATE {
            let create = &*(lparam.0 as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, create.lpCreateParams as isize);
        }
        if msg == WM_NCDESTROY {
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            return DefWindowProcW(hwnd, msg, wparam, lparam);
        }
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const RefCell<App>;
        if !ptr.is_null() {
            let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if let Ok(mut app) = (*ptr).try_borrow_mut() {
                    match app.message(hwnd, msg, wparam, lparam) {
                        Ok(value) => value,
                        Err(_) => {
                            app.report_error();
                            Some(LRESULT(0))
                        }
                    }
                } else {
                    None
                }
            }));
            match outcome {
                Ok(Some(result)) => return result,
                Err(_) => {
                    PostQuitMessage(1);
                    return LRESULT(0);
                }
                _ => {}
            }
        }
        DefWindowProcW(hwnd, msg, wparam, lparam)
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn wide_copy<const N: usize>(out: &mut [u16; N], value: &str) {
    for (target, c) in out.iter_mut().take(N - 1).zip(value.encode_utf16()) {
        *target = c;
    }
}
fn today() -> Result<i64> {
    let mut buffer = [0u16; 16];
    // SAFETY: explicit Gregorian locale, current local date and bounded output buffer.
    let count = unsafe {
        GetDateFormatEx(
            w!("en-US"),
            ENUM_DATE_FORMATS_FLAGS(0),
            None,
            w!("yyyyMMdd"),
            Some(&mut buffer),
            None,
        )
    };
    if count != 9 {
        return Err(Error::from_thread().into());
    }
    let number: i64 = String::from_utf16(&buffer[..8])?.parse()?;
    Ok(civil_day(number / 10000, number / 100 % 100, number % 100))
}
fn civil_day(mut year: i64, month: i64, day: i64) -> i64 {
    year -= i64::from(month <= 2);
    let era = year.div_euclid(400);
    let y = year - era * 400;
    let m = month + if month > 2 { -3 } else { 9 };
    era * 146097 + y * 365 + y / 4 - y / 100 + (153 * m + 2) / 5 + day - 1
}
fn monitor_work(rect: RECT) -> Result<RECT> {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    unsafe {
        if !GetMonitorInfoW(MonitorFromRect(&rect, MONITOR_DEFAULTTONEAREST), &mut info).as_bool() {
            return Err(Error::from_thread().into());
        }
    }
    Ok(info.rcWork)
}

struct MutexHandle(HANDLE);
impl Drop for MutexHandle {
    fn drop(&mut self) {
        unsafe {
            let _ = CloseHandle(self.0);
        }
    }
}
struct Windows {
    pet: HWND,
    panel: HWND,
    tray: bool,
    hotkey: bool,
}
impl Drop for Windows {
    fn drop(&mut self) {
        // SAFETY: all registrations/resources are removed before the boxed callback state drops.
        unsafe {
            if self.hotkey {
                let _ = UnregisterHotKey(Some(self.pet), HOTKEY);
            }
            let _ = KillTimer(Some(self.pet), 1);
            if self.tray {
                let data = NOTIFYICONDATAW {
                    cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
                    hWnd: self.pet,
                    uID: 1,
                    ..Default::default()
                };
                let _ = Shell_NotifyIconW(NIM_DELETE, &data);
            }
            if !self.panel.is_invalid() {
                let _ = DestroyWindow(self.panel);
            }
            if !self.pet.is_invalid() {
                let _ = DestroyWindow(self.pet);
            }
        }
    }
}

pub fn run() -> Result<()> {
    if cfg!(not(all(target_os = "windows", target_arch = "x86_64"))) {
        return Err("only Windows x64 is supported".into());
    }
    let mutex_name = wide(&format!(
        "Local\\com.ayangweb.bongo-cat.{}.personal",
        storage::ENVIRONMENT
    ));
    // SAFETY: owned named mutex handle, no shared data pointer, released on every return path.
    let mutex = MutexHandle(unsafe { CreateMutexW(None, false, PCWSTR(mutex_name.as_ptr()))? });
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return Ok(());
    }
    let _mutex = mutex;
    unsafe {
        let _ = SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    }
    let path = storage::state_path()?;
    let state = storage::load(&path, today()?)?;
    let avatar = image::load_from_memory(include_bytes!("../../assets/avatar.png"))?.into_rgba8();
    let args: Vec<String> = std::env::args().collect();
    let smoke_seconds = args
        .windows(2)
        .find(|pair| pair[0] == "--run-seconds")
        .and_then(|pair| pair[1].parse::<u64>().ok())
        .filter(|v| *v > 0);
    let mut app = Box::new(RefCell::new(App {
        timer: Timer::restored(state.total_millis),
        saved: state.clone(),
        state,
        path,
        avatar,
        pet: HWND::default(),
        panel: HWND::default(),
        dpi: 96,
        wheel: 0,
        drag: None,
        started: Instant::now(),
        hover: false,
        panel_running: false,
        panel_key: (0, 0, 0),
        last_saved: 0,
        save_failed: false,
        smoke_seconds,
    }));
    let mut windows = Windows {
        pet: HWND::default(),
        panel: HWND::default(),
        tray: false,
        hotkey: false,
    };
    // SAFETY: classes and windows are created/used on this thread; the boxed state remains
    // stable until Windows' destructor has destroyed both HWNDs and cleared userdata.
    unsafe {
        let instance = HINSTANCE(GetModuleHandleW(None)?.0);
        let class = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: w!("BongoCatPersonalStatic"),
            hCursor: LoadCursorW(None, IDC_ARROW)?,
            ..Default::default()
        };
        if RegisterClassW(&class) == 0 {
            return Err(Error::from_thread().into());
        }
        let ptr = app.as_ref() as *const RefCell<App> as *const std::ffi::c_void;
        let initial = app.get_mut().state.position.unwrap_or((100, 100));
        windows.pet = CreateWindowExW(
            WS_EX_LAYERED
                | if args.iter().any(|arg| arg == "--ui-smoke") {
                    WS_EX_APPWINDOW
                } else {
                    WS_EX_TOOLWINDOW
                },
            class.lpszClassName,
            w!("BongoCat"),
            WS_POPUP,
            initial.0,
            initial.1,
            350,
            200,
            None,
            None,
            Some(instance),
            Some(ptr),
        )?;
        app.get_mut().pet = windows.pet;
        windows.panel = CreateWindowExW(
            WS_EX_LAYERED | WS_EX_TOOLWINDOW,
            class.lpszClassName,
            w!("我的时光"),
            WS_POPUP,
            0,
            0,
            328,
            228,
            Some(windows.pet),
            None,
            Some(instance),
            Some(ptr),
        )?;
        app.get_mut().panel = windows.panel;
        let mut app = app.borrow_mut();
        app.dpi = GetDpiForWindow(windows.pet).max(96);
        let work = monitor_work(RECT {
            left: initial.0,
            top: initial.1,
            right: initial.0 + 350,
            bottom: initial.1 + 200,
        })?;
        let width = (350 * u32::from(app.state.scale) * app.dpi / 9600) as i32;
        let height =
            (width as f64 * f64::from(app.avatar.height()) / f64::from(app.avatar.width())) as i32;
        let (x, y) = app
            .state
            .position
            .unwrap_or((work.right - width - 40, work.bottom - height - 40));
        app.state.position = Some((
            x.clamp(work.left, (work.right - width).max(work.left)),
            y.clamp(work.top, (work.bottom - height).max(work.top)),
        ));
        app.redraw_pet()?;
        SetWindowPos(
            windows.pet,
            Some(if app.state.always_on_top {
                HWND_TOPMOST
            } else {
                HWND_NOTOPMOST
            }),
            0,
            0,
            0,
            0,
            SWP_NOSIZE | SWP_NOMOVE | SWP_NOACTIVATE,
        )?;
        let _ = ShowWindow(windows.pet, SW_SHOWNOACTIVATE);
        RegisterHotKey(
            Some(windows.pet),
            HOTKEY,
            MOD_CONTROL | MOD_ALT | MOD_NOREPEAT,
            u32::from(b'L'),
        )?;
        windows.hotkey = true;
        let mut tray = NOTIFYICONDATAW {
            cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
            hWnd: windows.pet,
            uID: 1,
            uFlags: NIF_MESSAGE | NIF_ICON | NIF_TIP,
            uCallbackMessage: TRAY_MESSAGE,
            hIcon: LoadIconW(None, IDI_APPLICATION)?,
            ..Default::default()
        };
        wide_copy(&mut tray.szTip, "BongoCat · Ctrl+Alt+L 开始 / 停止");
        if !Shell_NotifyIconW(NIM_ADD, &tray).as_bool() {
            return Err(Error::from_thread().into());
        }
        windows.tray = true;
        if SetTimer(Some(windows.pet), 1, 1000, None) == 0 {
            return Err(Error::from_thread().into());
        }
        app.persist()?;
        if args.iter().any(|arg| arg == "--show-panel") {
            app.toggle_panel()?;
        }
        drop(app);
        let mut message = MSG::default();
        loop {
            let result = GetMessageW(&mut message, None, 0, 0).0;
            if result == -1 {
                return Err(Error::from_thread().into());
            }
            if result == 0 {
                break;
            }
            let _ = TranslateMessage(&message);
            DispatchMessageW(&message);
        }
    }
    let now = app.borrow().now();
    app.get_mut().timer.stop(now);
    app.get_mut().persist()?;
    drop(windows);
    Ok(())
}

pub fn show_startup_error(message: &str) {
    let text = wide(&format!(
        "桌宠无法启动。请根据下方原因检查后重试。\n\n{message}"
    ));
    unsafe {
        MessageBoxW(
            None,
            PCWSTR(text.as_ptr()),
            w!("BongoCat"),
            MB_OK | MB_ICONERROR,
        );
    }
}

pub fn data_root() -> std::io::Result<PathBuf> {
    // SAFETY: the shell returns a COM-allocated string, copied before freeing it exactly once.
    unsafe {
        let value = SHGetKnownFolderPath(&FOLDERID_RoamingAppData, KF_FLAG_DEFAULT, None)
            .map_err(std::io::Error::other)?;
        let text = value.to_string().map_err(std::io::Error::other);
        ::windows::Win32::System::Com::CoTaskMemFree(Some(value.0.cast()));
        text.map(PathBuf::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn calendar_boundaries() {
        assert_eq!(civil_day(2024, 3, 1) - civil_day(2024, 2, 28), 2);
        assert_eq!(civil_day(2025, 1, 1) - civil_day(2024, 12, 31), 1);
        assert!(today().is_ok());
    }
}
