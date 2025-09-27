use std::sync::Arc;
use std::sync::Mutex;

use pinnacle_api::input;
use pinnacle_api::input::Bind;
use pinnacle_api::input::Keysym;
use pinnacle_api::input::libinput::AccelProfile;
use pinnacle_api::input::libinput::ClickMethod;
use pinnacle_api::input::libinput::DeviceHandle;
use pinnacle_api::input::{Mod, MouseButton};
use pinnacle_api::layout;
use pinnacle_api::layout::LayoutGenerator;
use pinnacle_api::layout::LayoutNode;
use pinnacle_api::layout::LayoutRequester;
use pinnacle_api::layout::LayoutResponse;
use pinnacle_api::layout::generators::Cycle;
use pinnacle_api::layout::generators::MasterStack;
use pinnacle_api::output;
use pinnacle_api::process::Command;
use pinnacle_api::signal::InputSignal;
use pinnacle_api::signal::OutputSignal;
use pinnacle_api::tag;
use pinnacle_api::util::Batch;
use pinnacle_api::util::Direction;
use pinnacle_api::window;
use pinnacle_api::window::VrrDemand;
use pinnacle_api::window::WindowHandle;

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

    // `mod_key + ctrl + space` toggles floating
    input::keybind(mod_key | Mod::CTRL, Keysym::space)
        .on_press(|| {
            if let Some(window) = window::get_focused() {
                window.toggle_floating();
                window.raise();
            }
        })
        .group("Window")
        .description("Toggle floating on the focused window");

    // `mod_key + f` toggles fullscreen
    input::keybind(mod_key, 'f')
        .on_press(|| {
            if let Some(window) = window::get_focused() {
                window.toggle_fullscreen();
                window.raise();
            }
        })
        .group("Window")
        .description("Toggle fullscreen on the focused window");

    // `mod_key + m` toggles maximized
    input::keybind(mod_key, 'm')
        .on_press(|| {
            if let Some(window) = window::get_focused() {
                window.toggle_maximized();
                window.raise();
            }
        })
        .group("Window")
        .description("Toggle maximized on the focused window");

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

    enum CircleDirection {
        Clockwise,
        CounterClockwise,
    }

    struct Circularized {
        forward: Direction,
        forward_cross: Direction,
        backward: Direction,
        backward_cross: Direction,
    }

    fn circularize_direction(cdir: CircleDirection) -> Circularized {
        match cdir {
            CircleDirection::Clockwise => Circularized {
                forward: Direction::Right,
                forward_cross: Direction::Down,
                backward: Direction::Left,
                backward_cross: Direction::Up,
            },
            CircleDirection::CounterClockwise => Circularized {
                forward: Direction::Up,
                forward_cross: Direction::Left,
                backward: Direction::Down,
                backward_cross: Direction::Right,
            },
        }
    }

    fn on_next_circular(
        focused: Option<WindowHandle>,
        circle: CircleDirection,
        action: impl FnOnce(&WindowHandle, &WindowHandle),
    ) {
        if let Some(focused) = focused {
            let Circularized {
                forward,
                forward_cross,
                backward,
                backward_cross,
            } = circularize_direction(circle);

            let mut reversed: Vec<_> = focused.in_direction(backward).collect();
            let mut reversed_cross: Vec<_> = focused.in_direction(backward_cross).collect();
            reversed.reverse();
            reversed_cross.reverse();

            if let Some(next) = focused
                // build a circular iterator by chaining an iterator in the four cardinal directions
                // when there are only a few windows (i.e. most of the time), most of these directions will yield nothing
                // except for one.
                .in_direction(forward)
                .chain(focused.in_direction(forward_cross))
                .chain(reversed)
                .chain(reversed_cross)
                // fallback if directional movement doesn't yield anything -- mainly needed to be able to rotate maximized windows
                .chain(
                    focused
                        .tags()
                        .flat_map(|tag| tag.windows().filter(|w| w != &focused)),
                )
                .next()
            {
                action(&focused, &next)
            }
        }
    }

    fn move_focus() -> impl Fn(&WindowHandle, &WindowHandle) {
        |focused: &WindowHandle, next: &WindowHandle| {
            if focused.maximized() {
                focused.lower();
                next.set_maximized(true);
                next.raise();
            }
            next.set_focused(true);
        }
    }

    input::keybind(mod_key, 'j')
        .on_press(|| {
            on_next_circular(
                window::get_focused(),
                CircleDirection::Clockwise,
                move_focus(),
            );
        })
        .group("Window")
        .description("focus next window");

    input::keybind(mod_key, Keysym::Tab)
        .on_press(|| {
            on_next_circular(
                window::get_focused(),
                CircleDirection::Clockwise,
                move_focus(),
            );
        })
        .group("Window")
        .description("focus prev window");

    input::keybind(mod_key, 'k')
        .on_press(|| {
            on_next_circular(
                window::get_focused(),
                CircleDirection::CounterClockwise,
                move_focus(),
            );
        })
        .group("Window")
        .description("focus prev window");

    input::keybind(mod_key | Mod::SHIFT, Keysym::Tab)
        .on_press(|| {
            on_next_circular(
                window::get_focused(),
                CircleDirection::CounterClockwise,
                move_focus(),
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

    fn swap_windows(
        layout_requester: &LayoutRequester,
    ) -> impl FnOnce(&WindowHandle, &WindowHandle) {
        let requester = layout_requester.clone();
        move |focused, next| {
            focused.swap(next);
            focused.set_focused(true);
            requester.request_layout();
        }
    }

    input::keybind(mod_key | Mod::SHIFT, 'j')
        .on_press({
            let requester = layout_requester.clone();
            move || {
                on_next_circular(
                    window::get_focused(),
                    CircleDirection::Clockwise,
                    swap_windows(&requester),
                );
            }
        })
        .group("Window")
        .description("shift window forward");

    input::keybind(mod_key | Mod::SHIFT, 'k')
        .on_press({
            let requester = layout_requester.clone();
            move || {
                on_next_circular(
                    window::get_focused(),
                    CircleDirection::CounterClockwise,
                    swap_windows(&requester),
                );
            }
        })
        .group("Window")
        .description("shift window backwards");

    input::keybind(mod_key, 'h')
        .on_press({
            let requester = layout_requester.clone();
            move || {
                if let Some(focused) = window::get_focused() {
                    let master = focused
                        .in_direction(Direction::Left)
                        .next()
                        .unwrap_or(focused);
                    master.resize_tile(0, -10, 0, 0);
                    requester.request_layout();
                }
            }
        })
        .group("Window")
        .description("decrease master pane size");

    input::keybind(mod_key, 'l')
        .on_press({
            let requester = layout_requester.clone();
            move || {
                if let Some(focused) = window::get_focused() {
                    let master = focused
                        .in_direction(Direction::Left)
                        .next()
                        .unwrap_or(focused);
                    master.resize_tile(0, 10, 0, 0);
                    requester.request_layout();
                }
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
            Command::new("emacsclient")
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
            Command::new("emacsclient")
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
        let output_name = output.name();
        let monitor = format!("monitor={output_name}");
        tags.next().unwrap().set_active(true);
        Command::new("eww")
            .args([
                "open",
                "--screen",
                &*output.name(),
                "primary",
                "--arg",
                &*monitor,
            ])
            .spawn();
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
            device.set_accel_profile(AccelProfile::Adaptive);
            device.set_accel_speed(0.75f64);
            device.set_natural_scroll(true);
            device.set_click_method(ClickMethod::Clickfinger);
        }
    }

    input::libinput::for_each_device(prep_devices);
    input::connect_signal(InputSignal::DeviceAdded(Box::new(prep_devices)));

    #[cfg(feature = "snowcap")]
    use pinnacle_api::{
        experimental::snowcap_api::{decoration::DecorationHandle, widget::Color},
        snowcap::{FocusBorder, FocusBorderMessage},
    };

    #[cfg(feature = "snowcap")]
    fn make_fb(win: &WindowHandle) -> DecorationHandle<FocusBorderMessage> {
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
    }

    fn apply_window_rules(window: WindowHandle) {
        window.set_decoration_mode(window::DecorationMode::ServerSide);

        #[cfg(feature = "snowcap")]
        make_fb(&window);

        window.set_vrr_demand(VrrDemand::when_fullscreen());

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
                    window.set_tags(tag::get("III"));
                } else {
                    window.set_maximized(true);
                    window.set_tags(tag::get("I"));
                }
            }
            _ => {}
        }
    }

    // Add borders to already existing windows.
    window::get_all().for_each(apply_window_rules);

    // Add borders to new windows.
    window::add_window_rule(apply_window_rules);

    // Focus outputs when the pointer enters them
    output::connect_signal(OutputSignal::PointerEnter(Box::new(|output| {
        output.focus();
    })));

    #[cfg(feature = "snowcap")]
    if let Some(error) = pinnacle_api::pinnacle::take_last_error() {
        // Show previous crash messages
        pinnacle_api::snowcap::ConfigCrashedMessage::new(error).show();
    }

    pinnacle_api::pinnacle::set_xwayland_self_scaling(true);

    Command::new("eww").args(["daemon"]).once().spawn();
    Command::new(terminal).once().spawn();
}

pinnacle_api::main!(config);
