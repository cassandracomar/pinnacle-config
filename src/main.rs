use std::sync::Arc;
use std::sync::Mutex;

use pinnacle_api::input;
use pinnacle_api::input::Bind;
use pinnacle_api::input::Keysym;
use pinnacle_api::input::{Mod, MouseButton};
use pinnacle_api::layout;
use pinnacle_api::layout::LayoutGenerator;
use pinnacle_api::layout::LayoutNode;
use pinnacle_api::layout::LayoutResponse;
use pinnacle_api::layout::generators::Cycle;
use pinnacle_api::layout::generators::MasterStack;
use pinnacle_api::output;
use pinnacle_api::pinnacle;
use pinnacle_api::pinnacle::Backend;
use pinnacle_api::process::Command;
use pinnacle_api::signal::OutputSignal;
use pinnacle_api::signal::WindowSignal;
use pinnacle_api::tag;
use pinnacle_api::util::Batch;
use pinnacle_api::util::Direction;
use pinnacle_api::window;

async fn config() {
    // Change the mod key to `Alt` when running as a nested window.
    let mod_key = match pinnacle::backend() {
        Backend::Tty => Mod::ALT,
        Backend::Window => Mod::ALT,
    };

    let terminal = "wezterm";

    //------------------------
    // Mousebinds            |
    //------------------------

    // `mod_key + left click` starts moving a window
    input::mousebind(mod_key, MouseButton::Left)
        .on_press(|| {
            window::begin_move(MouseButton::Left);
        })
        .group("Mouse")
        .description("Start an interactive window move");

    // `mod_key + right click` starts resizing a window
    input::mousebind(mod_key, MouseButton::Right)
        .on_press(|| {
            window::begin_resize(MouseButton::Right);
        })
        .group("Mouse")
        .description("Start an interactive window resize");

    //------------------------
    // Keybinds              |
    //------------------------

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

    // `mod_key + ctrl + r` reloads the config
    input::keybind(mod_key | Mod::CTRL, 'r')
        .set_as_reload_config()
        .group("Compositor")
        .description("Reload the config");

    // `mod_key + shift + c` closes the focused window
    input::keybind(mod_key | Mod::SHIFT, 'c')
        .on_press(|| {
            if let Some(window) = window::get_focused() {
                window.close();
            }
        })
        .group("Window")
        .description("Close the focused window");

    // `mod_key + Return` spawns a terminal
    input::keybind(mod_key, Keysym::Return)
        .on_press(move || {
            Command::new(terminal).spawn();
        })
        .group("Process")
        .description("Spawn a terminal");

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
                .args(["-show", "combi", "drun,run,ssh"])
                .spawn();
        })
        .group("Process")
        .description("spawn the application launcher");

    input::keybind(mod_key, 'j')
        .on_press(|| {
            if let Some(focused) = window::get_focused() {
                if let Some(closest_right) = focused.in_direction(Direction::Right).next() {
                    closest_right.set_focused(true);
                }
            }
        })
        .group("Window")
        .description("focus next window");

    input::keybind(mod_key, Keysym::Tab)
        .on_press(|| {
            if let Some(focused) = window::get_focused() {
                if let Some(closest_right) = focused.in_direction(Direction::Right).next() {
                    closest_right.set_focused(true);
                }
            }
        })
        .group("Window")
        .description("focus prev window");

    input::keybind(mod_key, 'k')
        .on_press(|| {
            if let Some(focused) = window::get_focused() {
                if let Some(closest_left) = focused.in_direction(Direction::Left).next() {
                    closest_left.set_focused(true);
                }
            }
        })
        .group("Window")
        .description("focus prev window");

    input::keybind(mod_key | Mod::SHIFT, Keysym::Tab)
        .on_press(|| {
            if let Some(focused) = window::get_focused() {
                if let Some(closest_left) = focused.in_direction(Direction::Left).next() {
                    closest_left.set_focused(true);
                }
            }
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

    let current_master_factor = Arc::new(Mutex::new(0.5f32));

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

    let cycler2 = cycler.clone();
    let master_factor_2 = current_master_factor.clone();
    let cycler3 = cycler.clone();
    let master_factor_3 = current_master_factor.clone();

    // `mod_key + shift + space` cycles to the previous layout
    input::keybind(mod_key | Mod::SHIFT, Keysym::space)
        .on_press(move || {
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
            layout_requester.request_layout_on_output(&focused_op);
        })
        .group("Layout")
        .description("Cycle the layout backward");

    input::keybind(mod_key, 'h')
        .on_press(move || {
            let mf = master_factor_2.clone();
            let master_factor = {
                let mut master_factor = mf.lock().unwrap();
                *master_factor -= 0.1;
                *master_factor
            };
            let c = &mut *cycler2.lock().unwrap();
            // add an API function to mutate layouts so you can maintain cycle position
            *c = Cycle::new([into_box(MasterStack {
                master_factor,
                ..Default::default()
            })]);
        })
        .group("Window")
        .description("decrease master pane size");

    input::keybind(mod_key, 'l')
        .on_press(move || {
            let mf = master_factor_3.clone();
            let master_factor = {
                let mut master_factor = mf.lock().unwrap();
                *master_factor += 0.1;
                *master_factor
            };
            let c = &mut *cycler3.lock().unwrap();
            // add an API function to mutate layouts so you can maintain cycle position
            *c = Cycle::new([into_box(MasterStack {
                master_factor,
                ..Default::default()
            })]);
        })
        .group("Window")
        .description("increase master pane size");
    //------------------------
    // Tags                  |
    //------------------------

    let tag_names = ["I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX"];

    // Setup all monitors with tags "1" through "9"
    output::for_each_output(move |output| {
        output.set_scale(2.0);

        let mut tags = tag::add(output, tag_names);
        tags.next().unwrap().set_active(true);
    });

    for (tag_name, index) in tag_names.into_iter().zip('1'..='9') {
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

    input::libinput::for_each_device(|device| {
        // Enable natural scroll for touchpads
        if device.device_type().is_touchpad() {
            device.set_natural_scroll(true);
        }
    });

    // There are no server-side decorations yet, so request all clients use client-side decorations.
    window::add_window_rule(|window| {
        window.set_decoration_mode(window::DecorationMode::ClientSide);
    });

    // Enable sloppy focus
    window::connect_signal(WindowSignal::PointerEnter(Box::new(|win| {
        win.set_focused(true);
    })));

    // Focus outputs when the pointer enters them
    output::connect_signal(OutputSignal::PointerEnter(Box::new(|output| {
        output.focus();
    })));

    #[cfg(feature = "snowcap")]
    if let Some(error) = pinnacle_api::pinnacle::take_last_error() {
        // Show previous crash messages
        pinnacle_api::snowcap::ConfigCrashedMessage::new(error).show();
    } else {
        // Or show the bind overlay on startup
        pinnacle_api::snowcap::BindOverlay::new().show();
    }

    Command::new(terminal).once().spawn();
}

pinnacle_api::main!(config);
