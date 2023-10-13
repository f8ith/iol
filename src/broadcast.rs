use chrono::Utc;
use glow::HasContext;
use imgui::{Condition, Context};
use imgui_glow_renderer::AutoRenderer;
use imgui_sdl2_support::SdlPlatform;
use iol::IolEvent;
use mio::Events;
use mio::{Interest, Poll, Token};
use postcard::from_bytes;
use postcard::to_vec;
use sdl2::{
    event::Event,
    video::{GLProfile, Window},
};
use std::collections::HashMap;
use std::io;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;

const SCREEN_WIDTH: u32 = 1280;
const SCREEN_HEIGHT: u32 = 720;

const UDP_SOCKET: Token = Token(0);

// Create a new glow context.
fn glow_context(window: &Window) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function(|s| window.subsystem().gl_get_proc_address(s) as _)
    }
}

fn main() -> io::Result<()> {
    use std::time::Duration;

    // Setup UDP
    use mio::net::UdpSocket;
    use sdl2::controller::GameController;

    env_logger::init();

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1);
    let addr = "0.0.0.0:5864".parse().unwrap();

    let mut socket = UdpSocket::bind(addr)?;
    poll.registry().register(
        &mut socket,
        UDP_SOCKET,
        Interest::WRITABLE | Interest::READABLE,
    )?;

    let mut buf = [0; 1 << 16];

    let mut broadcast_keyboard = true;
    let mut broadcast_gamepad = true;
    let mut server_address_str = "127.0.0.1:4863".to_owned();
    let mut server_address: SocketAddr =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 4863);

    'wait: loop {
        if let Err(err) = poll.poll(&mut events, None) {
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err);
        }

        for event in events.iter() {
            if event.token() == UDP_SOCKET {
                break 'wait;
            }
        }
    }

    /* initialize SDL and its video subsystem */
    let sdl = sdl2::init().unwrap();
    let video_subsystem = sdl.video().unwrap();
    let controller_subsystem = sdl.game_controller().unwrap();
    controller_subsystem.set_event_state(true);
    let mut controllers: HashMap<u32, GameController> = HashMap::new();
    let mut controllers_netids: HashMap<u32, u32> = HashMap::new();

    /* hint SDL to initialize an OpenGL 3.3 core profile context */
    let gl_attr = video_subsystem.gl_attr();

    gl_attr.set_context_version(3, 3);
    gl_attr.set_context_profile(GLProfile::Core);

    /* create a new window, be sure to call opengl method on the builder when using glow! */
    let window = video_subsystem
        .window("iol", SCREEN_WIDTH, SCREEN_HEIGHT)
        .allow_highdpi()
        .opengl()
        .position_centered()
        .resizable()
        .build()
        .unwrap();

    let gl_context = window.gl_create_context().unwrap();
    window.gl_make_current(&gl_context).unwrap();

    window.subsystem().gl_set_swap_interval(1).unwrap();

    let gl = glow_context(&window);
    let mut imgui = Context::create();

    imgui.set_ini_filename(None);
    imgui.set_log_filename(None);

    /* setup platform and renderer, and fonts to imgui */
    imgui
        .fonts()
        .add_font(&[imgui::FontSource::DefaultFontData { config: None }]);

    let mut platform = SdlPlatform::init(&mut imgui);
    let mut renderer = AutoRenderer::initialize(gl, &mut imgui).unwrap();

    let mut event_pump = sdl.event_pump().unwrap();

    'main: loop {
        // Process each event.

        for event in event_pump.poll_iter() {
            /* pass all events to imgui platfrom */
            platform.handle_event(&mut imgui, &event);

            match event {
                Event::Quit { .. } => break 'main,
                Event::KeyDown {
                    scancode, repeat, ..
                } => {
                    if !repeat {
                        println!("{}: Scancode {:?} was pressed.", Utc::now(), scancode);
                        if broadcast_keyboard {
                            let serialized = to_vec::<IolEvent, 32>(&IolEvent::KeyDown {
                                scancode: scancode.unwrap(),
                                repeat,
                            })
                            .unwrap();

                            socket.send_to(serialized.as_slice(), server_address)?;
                        }
                    }
                }
                Event::KeyUp { scancode, .. } => {
                    println!("{:?}: Scancode {:?} was released.", Utc::now(), scancode);
                    if broadcast_keyboard {
                        let serialized = to_vec::<IolEvent, 32>(&IolEvent::KeyUp {
                            scancode: scancode.unwrap(),
                        })
                        .unwrap();

                        socket.send_to(serialized.as_slice(), server_address)?;
                    }
                }
                Event::ControllerDeviceAdded { which, .. } => {
                    println!("Controller with index {} was added.", which);

                    match controller_subsystem.open(which) {
                        Ok(c) => {
                            println!("Success: opened \"{}\"", c.name());

                            controllers.insert(which, c);
                        }
                        Err(e) => {
                            println!("failed: {:?}", e);
                        }
                    }
                }

                Event::ControllerDeviceRemoved { which, .. } => {
                    println!("Controller with index {} was removed.", which);
                    controllers.remove(&which);

                    //TODO: Remove controller from server.
                }
                Event::ControllerButtonDown { which, button, .. } => {
                    println!(
                        "{:?}: Controller with index {} pressed {:?}.",
                        Utc::now(),
                        which,
                        button
                    );
                    let id = controllers_netids.get(&which);
                    if let Some(id) = id {
                        let serialized =
                            to_vec::<IolEvent, 32>(&IolEvent::ButtonDown { id: *id, button })
                                .unwrap();
                        socket.send_to(serialized.as_slice(), server_address)?;
                    }
                }
                Event::ControllerButtonUp { which, button, .. } => {
                    println!(
                        "{:?}: Controller with index {} released {:?}.",
                        Utc::now(),
                        which,
                        button
                    );
                    let id = controllers_netids.get(&which);
                    if let Some(id) = id {
                        let serialized =
                            to_vec::<IolEvent, 32>(&IolEvent::ButtonUp { id: *id, button })
                                .unwrap();
                        socket.send_to(serialized.as_slice(), server_address)?;
                    }
                }
                Event::ControllerAxisMotion {
                    which, axis, value, ..
                } => {
                    println!(
                        "Controller with index {} moved axis {:?} to {:?}.",
                        which, axis, value
                    );

                    let id = controllers_netids.get(&which);
                    if let Some(id) = id {
                        let serialized = to_vec::<IolEvent, 32>(&IolEvent::AxisMotion {
                            id: *id,
                            axis,
                            value,
                        })
                        .unwrap();
                        socket.send_to(serialized.as_slice(), server_address)?;
                    }
                }
                _ => {}
            }
        }

        if broadcast_gamepad {}

        platform.prepare_frame(&mut imgui, &window, &event_pump);
        let ui = imgui.new_frame();

        ui.show_demo_window(&mut true);
        ui.window("Receivers")
            .size([300.0, 300.0], Condition::FirstUseEver)
            .build(|| {
                ui.checkbox("Keyboard", &mut broadcast_keyboard);
                ui.same_line();
                ui.checkbox("Gamepad", &mut broadcast_gamepad);
                ui.spacing();
                ui.dummy([0.0, 20.0]);

                ui.input_text("Server Address", &mut server_address_str)
                    .build();
                if ui.button("Connect") {
                    server_address = server_address_str
                        .parse()
                        .expect("Unable to parse socket address");
                    for controller in controllers.iter() {
                        let serialized = to_vec::<IolEvent, 32>(&IolEvent::PhysicalDeviceAdded {
                            which: controller.1.instance_id(),
                        })
                        .unwrap();
                        socket.send_to(serialized.as_slice(), server_address).ok();

                        'listen: loop {
                            println!("here");
                            if let Err(err) = poll.poll(&mut events, Some(Duration::from_secs(5))) {
                                if err.kind() == io::ErrorKind::Interrupted {
                                    continue 'listen;
                                }
                                return Err(err);
                            }
                            match socket.recv_from(&mut buf) {
                                Ok((packet_size, _)) => {
                                    let event =
                                        from_bytes::<IolEvent>(&buf[..packet_size]).unwrap();

                                    match event {
                                        IolEvent::VirtualDeviceAdded { id, which } => {
                                            println!(
                                            "Controller with index {} was added on the listener.",
                                            id
                                        );
                                            controllers_netids.insert(which, id);
                                            break 'listen;
                                        }
                                        _ => {}
                                    }
                                }
                                Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                                    break 'listen;
                                }
                                Err(e) => {
                                    return Err(e);
                                }
                            }
                        }
                    }
                }
                Ok(())
            });

        ui.window("Controllers")
            .size([300.0, 300.0], Condition::FirstUseEver)
            .position([60.0, 400.0], Condition::FirstUseEver)
            .build(|| {
                let clipper = imgui::ListClipper::new(controllers.len() as i32)
                    .items_height(ui.current_font_size())
                    .begin(ui);
                let mut controller_iter = controllers.iter();
                for _ in clipper.iter() {
                    ui.text(controller_iter.next().unwrap().1.name());
                }
            });

        let draw_data = imgui.render();

        unsafe { renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };
        renderer.render(draw_data).unwrap();

        window.gl_swap_window();
    }

    Ok(())
}
