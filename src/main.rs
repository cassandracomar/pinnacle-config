use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use pinnacle_api::input;
use pinnacle_api::input::Bind;
use pinnacle_api::input::Keysym;
use pinnacle_api::input::libinput::AccelProfile;
use pinnacle_api::input::libinput::ClickMethod;
use pinnacle_api::input::libinput::DeviceHandle;
use pinnacle_api::input::libinput::TapButtonMap;
use pinnacle_api::input::{Mod, MouseButton};
use pinnacle_api::layout;
use pinnacle_api::layout::LayoutGenerator;
use pinnacle_api::layout::LayoutNode;
use pinnacle_api::layout::LayoutResponse;
use pinnacle_api::layout::generators::Cycle;
use pinnacle_api::layout::generators::MasterStack;
use pinnacle_api::output;
use pinnacle_api::process::Command;
use pinnacle_api::signal::InputSignal;
use pinnacle_api::signal::OutputSignal;
use pinnacle_api::signal::WindowSignal;
use pinnacle_api::tag;
use pinnacle_api::util::Batch;
use pinnacle_api::util::Direction;
use pinnacle_api::window;
use pinnacle_api::window::VrrDemand;
use pinnacle_api::window::WindowHandle;

#[cfg(feature = "snowcap")]
use pinnacle_api::{experimental::snowcap_api::widget::Color, snowcap::FocusBorder};
use tokio::time::sleep;

use crate::uwsm_command::UwsmCommand;
use crate::zipper::SequenceDirection;
use crate::zipper::Zipper;

pub mod uwsm_command;
pub mod zipper;

fn cycle_next(
    focused: Option<WindowHandle>,
    dir: SequenceDirection,
    action: impl FnOnce(&WindowHandle, &WindowHandle),
) {
    if let Some(focused) = focused {
        let zipper = focused
            .tags()
            .flat_map(|tag| tag.windows())
            .collect::<Zipper<_>>()
            .refocus(|t| t == &focused);

        if let Some(next) = zipper.circle_step(dir).focus() {
            action(&focused, next)
        }
    }
}

#[cfg(feature = "snowcap")]
fn make_fb(win: &WindowHandle) {
    FocusBorder {
        unfocused_color: Color::rgb(
            (0x3c as f32) / (0xff as f32),
            (0x2c as f32) / (0xff as f32),
            (0x1c as f32) / (0xff as f32),
        ),
        focused_color: Color::rgb(
            (0xee as f32) / (0xff as f32),
            (0xde as f32) / (0xff as f32),
            (0xce as f32) / (0xff as f32),
        ),
        thickness: 2,
        ..FocusBorder::new(win)
    }
    .decorate()
    .map_or_else(
        |err| {
            println!("failed to decorate window: {err}");
        },
        |_| (),
    )
}

fn move_focus(focused: &WindowHandle, next: &WindowHandle) {
    if focused.maximized() || next.maximized() {
        focused.lower();
        next.set_maximized(true);
        next.raise();
    }
    next.set_focused(true);
}

fn swap_windows(focused: &WindowHandle, next: &WindowHandle) {
    focused.swap(next);
    focused.set_focused(true);
}

/// `config` sets up the pinnacle configuration via the `pinnacle_api`
async fn config() {
    // Change the mod key to `Alt` when running as a nested window.
    let mod_key = Mod::ALT;
    let mod4_key = Mod::SUPER;

    let terminal = "wezterm";

    //------------------------
    // Mousebinds            |
    //------------------------

    // `mod_key + left click` starts moving a window
    input::mousebind(mod_key, MouseButton::Left)
        .on_press(|| {
            if let Some(w) = window::get_focused() {
                w.set_floating(true);
                w.raise();
            }
            window::begin_move(MouseButton::Left);
        })
        .group("Mouse")
        .description("Start an interactive window move");

    // `mod_key + Shift + left click` unfloats the window
    input::mousebind(mod_key | Mod::SHIFT, MouseButton::Left)
        .on_press(|| {
            if let Some(w) = window::get_focused() {
                w.set_floating(false);
                w.lower();
            }
            window::begin_move(MouseButton::Left);
        })
        .group("Mouse")
        .description("Start an interactive window move");

    // `mod_key + right click` starts resizing a window
    input::mousebind(mod_key, MouseButton::Right)
        .on_press(|| {
            if let Some(w) = window::get_focused() {
                w.set_floating(true);
                w.raise();
            }
            window::begin_resize(MouseButton::Right);
        })
        .group("Mouse")
        .description("Start an interactive window resize");

    //------------------------
    // Keybinds              |
    //------------------------

    input::keybind(mod_key, 't')
        .on_press(|| {
            if let Some(w) = window::get_focused() {
                w.toggle_floating();
            }
        })
        .group("Window")
        .description("Toggle floating");

    // `mod_key + s` shows the bindings overlay
    #[cfg(feature = "snowcap")]
    input::keybind(mod_key, 's')
        .on_press(|| {
            pinnacle_api::snowcap::BindOverlay::new().show();
        })
        .group("Compositor")
        .description("Show the bindings overlay");

    // `mod_key + shift + q` quits Pinnacle
    #[cfg(not(feature = "snowcap"))]
    input::keybind(mod_key | Mod::SHIFT, 'q')
        .set_as_quit()
        .group("Compositor")
        .description("Quit Pinnacle");

    // mod + q reloads the config
    input::keybind(mod_key, 'q')
        .set_as_reload_config()
        .group("Compositor")
        .description("Reload Pinnacle Config");

    #[cfg(feature = "snowcap")]
    {
        // `mod_key + shift + q` shows the quit prompt
        input::keybind(mod_key | Mod::SHIFT, 'q')
            .on_press(|| {
                pinnacle_api::snowcap::QuitPrompt::new().show();
            })
            .group("Compositor")
            .description("Show quit prompt");

        // `mod_key + ctrl + shift + q` for the hard shutdown
        input::keybind(mod_key | Mod::CTRL | Mod::SHIFT, 'q')
            .set_as_quit()
            .group("Compositor")
            .description("Quit Pinnacle without prompt");
    }

    // `mod_key + shift + c` closes the focused window
    input::keybind(mod_key | Mod::SHIFT, 'c')
        .on_press(|| {
            if let Some(window) = window::get_focused() {
                window.close();
            }
        })
        .group("Window")
        .description("Close the focused window");

    input::keybind(mod_key | Mod::SHIFT, 'p')
        .on_press(move || {
            Command::new("clipcat-menu").spawn();
        })
        .group("Process")
        .description("Open Clipboard History");

    input::keybind(mod_key, 'o')
        .on_press(move || {
            Command::new("rofi-rbw").spawn();
        })
        .group("Process")
        .description("Bitwarden Passwords");

    input::keybind(mod_key, 'p')
        .on_press(|| {
            Command::new("rofi")
                .args([
                    "-show",
                    "combi",
                    "-modes",
                    "combi",
                    "-combi-modes",
                    "drun,run,calc,window,ssh",
                ])
                .spawn();
        })
        .group("Process")
        .description("spawn the application launcher");

    input::keybind(mod_key, 'n')
        .on_press(|| {
            Command::new("rofi")
                .args(["-show", "emoji", "-modes", "emoji"])
                .spawn();
        })
        .group("Process")
        .description("spawn the application launcher");

    input::keybind(mod_key, 'i')
        .on_press(|| {
            Command::new("rofi")
                .args([
                    "-show",
                    "file-browser-extended",
                    "-modes",
                    "file-browser-extended",
                ])
                .spawn();
        })
        .group("Process")
        .description("spawn the application launcher");

    input::keybind(mod4_key, 'p')
        .on_press(|| {
            Command::new("rofi-screenshot").spawn();
        })
        .group("Process")
        .description("take a screenshot");

    input::keybind(mod_key, 'j')
        .on_press(|| {
            cycle_next(
                window::get_focused(),
                SequenceDirection::Original,
                move_focus,
            );
        })
        .group("Window")
        .description("focus next window");

    input::keybind(mod_key, Keysym::Tab)
        .on_press(|| {
            cycle_next(
                window::get_focused(),
                SequenceDirection::Original,
                move_focus,
            );
        })
        .group("Window")
        .description("focus prev window");

    input::keybind(mod_key, 'k')
        .on_press(|| {
            cycle_next(
                window::get_focused(),
                SequenceDirection::Reverse,
                move_focus,
            );
        })
        .group("Window")
        .description("focus prev window");

    input::keybind(mod_key | Mod::SHIFT, Keysym::Tab)
        .on_press(|| {
            cycle_next(
                window::get_focused(),
                SequenceDirection::Reverse,
                move_focus,
            );
        })
        .group("Window")
        .description("focus next window");

    //------------------------
    // Layouts               |
    //------------------------

    // Pinnacle supports a tree-based layout system built on layout nodes.
    //
    // To determine the tree used to layout windows, Pinnacle requests your config for a tree data structure
    // with nodes containing gaps, directions, etc. There are a few provided utilities for creating
    // a layout, known as layout generators.
    //
    // ### Layout generators ###
    // A layout generator is a table that holds some state as well as
    // the `layout` function, which takes in a window count and computes
    // a tree of layout nodes that determines how windows are laid out.
    //
    // There are currently six built-in layout generators, one of which delegates to other
    // generators as shown below.

    fn into_box<'a, T: LayoutGenerator + Send + 'a>(
        generator: T,
    ) -> Box<dyn LayoutGenerator + Send + 'a> {
        Box::new(generator) as _
    }

    // Create a cycling layout generator that can cycle between layouts on different tags.
    let cycler = Arc::new(Mutex::new(Cycle::new([into_box(MasterStack::default())])));

    // Use the cycling layout generator to manage layout requests.
    // This returns a layout requester that allows you to request layouts manually.
    let layout_requester = layout::manage({
        let cycler = cycler.clone();
        move |args| {
            let Some(tag) = args.tags.first() else {
                return LayoutResponse {
                    root_node: LayoutNode::new(),
                    tree_id: 0,
                };
            };

            let mut cycler = cycler.lock().unwrap();
            cycler.set_current_tag(tag.clone());

            let root_node = cycler.layout(args.window_count);
            let tree_id = cycler.current_tree_id();
            LayoutResponse { root_node, tree_id }
        }
    });

    // `mod_key + ctrl + space` toggles floating
    input::keybind(mod_key | Mod::CTRL, Keysym::space)
        .on_press(move || {
            if let Some(window) = window::get_focused() {
                window.toggle_floating();
                window.raise();
            }
        })
        .group("Window")
        .description("Toggle floating on the focused window");

    // `mod_key + f` toggles fullscreen
    input::keybind(mod_key, 'f')
        .on_press(move || {
            if let Some(window) = window::get_focused() {
                window.toggle_fullscreen();
                window.raise();
            }
        })
        .group("Window")
        .description("Toggle fullscreen on the focused window");

    // `mod_key + m` toggles maximized
    input::keybind(mod_key, 'm')
        .on_press(move || {
            if let Some(window) = window::get_focused() {
                window.toggle_maximized();
                window.raise();
            }
        })
        .group("Window")
        .description("Toggle maximized on the focused window");

    // `mod_key + space` cycles to the next layout
    input::keybind(mod_key, Keysym::space)
        .on_press({
            let cycler = cycler.clone();
            let requester = layout_requester.clone();
            move || {
                let Some(focused_op) = output::get_focused() else {
                    return;
                };
                let Some(first_active_tag) = focused_op
                    .tags()
                    .batch_find(|tag| Box::pin(tag.active_async()), |active| *active)
                else {
                    return;
                };

                cycler
                    .lock()
                    .unwrap()
                    .cycle_layout_forward(&first_active_tag);
                requester.request_layout_on_output(&focused_op);
            }
        })
        .group("Layout")
        .description("Cycle the layout forward");

    // `mod_key + shift + space` cycles to the previous layout
    input::keybind(mod_key | Mod::SHIFT, Keysym::space)
        .on_press({
            let cycler = cycler.clone();
            let requester = layout_requester.clone();
            move || {
                let Some(focused_op) = output::get_focused() else {
                    return;
                };
                let Some(first_active_tag) = focused_op
                    .tags()
                    .batch_find(|tag| Box::pin(tag.active_async()), |active| *active)
                else {
                    return;
                };

                cycler
                    .lock()
                    .unwrap()
                    .cycle_layout_backward(&first_active_tag);
                requester.request_layout_on_output(&focused_op);
            }
        })
        .group("Layout")
        .description("Cycle the layout backward");

    input::keybind(mod_key | Mod::SHIFT, 'j')
        .on_press(|| {
            cycle_next(
                window::get_focused(),
                SequenceDirection::Original,
                swap_windows,
            );
        })
        .group("Window")
        .description("shift window forward");

    input::keybind(mod_key | Mod::SHIFT, 'k')
        .on_press(|| {
            cycle_next(
                window::get_focused(),
                SequenceDirection::Reverse,
                swap_windows,
            );
        })
        .group("Window")
        .description("shift window backwards");

    input::keybind(mod_key, 'h')
        .on_press(move || {
            if let Some(focused) = window::get_focused() {
                let master = focused
                    .in_direction(Direction::Left)
                    .next()
                    .unwrap_or(focused);
                let resize = master
                    .output()
                    .and_then(|output| output.current_mode())
                    .map(|mode| (mode.size.w as i32) / 10)
                    .unwrap_or(384);
                master.resize_tile(0, -1 * resize, 0, 0);
            }
        })
        .group("Window")
        .description("decrease master pane size");

    input::keybind(mod_key, 'l')
        .on_press(move || {
            if let Some(focused) = window::get_focused() {
                let master = focused
                    .in_direction(Direction::Left)
                    .next()
                    .unwrap_or(focused);
                let resize = master
                    .output()
                    .and_then(|output| output.current_mode())
                    .map(|mode| (mode.size.w as i32) / 10)
                    .unwrap_or(384);
                master.resize_tile(0, resize, 0, 0);
            }
        })
        .group("Window")
        .description("increase master pane size");

    let terminal_frame_name = "(name . \"emacsclient\")";
    let mu4e_frame_name = "(name . \"mu4e\")";
    let fullscreen = "(fullscreen . fullheight)";
    let auto_raise = "(auto-raise . nil)";
    let auto_lower = "(auto-lower . nil)";
    let wait_for_wm = "(wait-for-wm . t)";

    // `M-S-RET` spawns an eat terminal
    input::keybind(mod_key | Mod::SHIFT, Keysym::Return)
        .on_press(move || {
            UwsmCommand::new("emacsclient")
                .args([
                    "-c",
                    "-F",
                    &*format!(
                        "({terminal_frame_name} {fullscreen} {auto_raise} {auto_lower} {wait_for_wm})"
                    ),
                    "-e",
                    "(+eat/here)",
                ])
                .spawn();
        })
        .group("Process")
        .description("Open an emacs terminal");

    // `M-RET` spawns mu4e
    input::keybind(mod_key, Keysym::Return)
        .on_press(move || {
            UwsmCommand::new("emacsclient")
                .args(["-c", "-F", &*format!("({mu4e_frame_name})"), "-e", "(mu4e)"])
                .spawn();
        })
        .group("Process")
        .description("Open mu4e");

    //------------------------
    // Tags                  |
    //------------------------

    let tag_names = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"];

    // Setup all monitors with tags "1" through "9"
    output::for_each_output(move |output| {
        output.set_mode(3840, 2160, 120000);
        output.set_scale(2.0);
        output.set_vrr(output::Vrr::OnDemand);

        let mut tags = tag::add(output, tag_names);
        tags.next().unwrap().set_active(true);
    });

    for (tag_name, index) in tag_names.into_iter().zip(('1'..='9').chain('0'..='0')) {
        // `mod_key + 1-9` switches to tag "1" to "9"
        input::keybind(mod_key, index)
            .on_press(move || {
                if let Some(tag) = tag::get(tag_name) {
                    tag.switch_to();
                }
            })
            .group("Tag")
            .description(format!("Switch to tag {tag_name}"));

        // `mod_key + ctrl + 1-9` toggles tag "1" to "9"
        input::keybind(mod_key | Mod::CTRL, index)
            .on_press(move || {
                if let Some(tag) = tag::get(tag_name) {
                    tag.toggle_active();
                }
            })
            .group("Tag")
            .description(format!("Toggle tag {tag_name}"));

        // `mod_key + shift + 1-9` moves the focused window to tag "1" to "9"
        input::keybind(mod_key | Mod::SHIFT, index)
            .on_press(move || {
                if let Some(tag) = tag::get(tag_name)
                    && let Some(win) = window::get_focused()
                {
                    win.move_to_tag(&tag);
                    tag.switch_to();
                }
            })
            .group("Tag")
            .description(format!("Move the focused window to tag {tag_name}"));

        // `mod_key + ctrl + shift + 1-9` toggles tag "1" to "9" on the focused window
        input::keybind(mod_key | Mod::CTRL | Mod::SHIFT, index)
            .on_press(move || {
                if let Some(tg) = tag::get(tag_name)
                    && let Some(win) = window::get_focused()
                {
                    win.toggle_tag(&tg);
                }
            })
            .group("Tag")
            .description(format!("Toggle tag {tag_name} on the focused window"));
    }

    fn prep_devices(device: &DeviceHandle) {
        // Enable natural scroll for touchpads
        if device.device_type().is_touchpad() {
            device.set_tap(true);
            device.set_natural_scroll(true);
            device.set_tap_drag(false);

            device.set_click_method(ClickMethod::Clickfinger);
            device.set_tap_button_map(TapButtonMap::LeftRightMiddle);
            device.set_accel_profile(AccelProfile::Flat);
            device.set_accel_speed(1.0f64);
        }
    }

    input::libinput::for_each_device(prep_devices);
    input::connect_signal(InputSignal::DeviceAdded(Box::new(prep_devices)));

    fn apply_window_rules(window: WindowHandle) {
        match &*window.app_id() {
            "firefox" => {
                window.set_maximized(true);
                window.set_tags(tag::get("II"));
            }
            "org.wezfurlong.wezterm" => {
                window.set_tags(tag::get("VI"));
            }
            "emacs" => {
                if window.title().contains("emacsclient") {
                    window.set_maximized(false);
                    window.set_fullscreen(false);
                    window.set_tags(tag::get("IV"));
                } else if window.title().contains("mu4e") {
                    window.set_maximized(true);
                    window.set_tags(tag::get("V"));
                } else {
                    window.set_maximized(true);
                    window.set_tags(tag::get("I"));
                }
            }
            "Slack" => {
                window.set_tags(tag::get("III"));
                window.set_maximized(true);
            }
            _ => {}
        }

        window.set_decoration_mode(window::DecorationMode::ServerSide);

        #[cfg(feature = "snowcap")]
        make_fb(&window);

        window.set_vrr_demand(VrrDemand::when_fullscreen());
    }

    // Add borders to new windows.
    window::add_window_rule({
        let requester = layout_requester.clone();
        move |win| {
            apply_window_rules(win);
            requester.request_layout();
        }
    });

    // Focus outputs when the pointer enters them
    output::connect_signal(OutputSignal::PointerEnter(Box::new(|output| {
        output.focus();
    })));

    window::connect_signal(WindowSignal::Created(Box::new({
        let requester = layout_requester.clone();
        move |_win| {
            requester.request_layout();
        }
    })));

    window::connect_signal(WindowSignal::Focused(Box::new({
        let requester = layout_requester.clone();
        move |_win| {
            requester.request_layout();
        }
    })));

    window::connect_signal(WindowSignal::LayoutModeChanged(Box::new({
        let requester = layout_requester.clone();
        move |_win, _layout_mode| {
            requester.request_layout();
        }
    })));

    window::connect_signal(WindowSignal::Destroyed(Box::new({
        let requester = layout_requester.clone();
        move |_win, _title, _appid| {
            requester.request_layout();
        }
    })));

    #[cfg(feature = "snowcap")]
    if let Some(error) = pinnacle_api::pinnacle::take_last_error() {
        // Show previous crash messages
        pinnacle_api::snowcap::ConfigCrashedMessage::new(error).show();
    }

    pinnacle_api::pinnacle::set_xwayland_self_scaling(true);

    // need to delay creating the bar to give the daemon a bit of time to start
    sleep(Duration::from_secs(1)).await;
    output::for_each_output(|output| {
        let output_name = output.name();
        let eww_service = format!("eww-open@{output_name}");
        Command::new("systemctl")
            .args(["start", "--user", &eww_service])
            .spawn();
    });

    UwsmCommand::new(terminal).unique().once().spawn();
    UwsmCommand::new("firefox").unique().once().spawn();

    // Add borders to already existing windows.
    window::get_all().for_each(apply_window_rules);
    layout_requester.request_layout();
}

pinnacle_api::main!(config);
