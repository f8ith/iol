use glow::HasContext;
use imgui::{Condition, Context};
use imgui_glow_renderer::AutoRenderer;
use imgui_sdl2_support::SdlPlatform;
use iol::IolEvent;
use mio::net::UdpSocket;
use mio::Events;
use mio::{Interest, Poll, Token};
use postcard::from_bytes;
use postcard::to_vec;
use sdl2::controller::GameController;
use sdl2::{
    event::Event,
    video::{GLProfile, Window},
};
use std::collections::HashMap;
use std::io;
use std::net::IpAddr;
use std::net::Ipv4Addr;
use std::net::SocketAddr;
use std::time::Duration;

const SCREEN_WIDTH: u32 = 1280;
const SCREEN_HEIGHT: u32 = 720;

const UDP_SOCKET: Token = Token(0);

// Create a new glow context.
fn glow_context(window: &Window) -> glow::Context {
    unsafe {
        glow::Context::from_loader_function(|s| window.subsystem().gl_get_proc_address(s) as _)
    }
}

fn setup_controller_id(
    controller: &GameController,
    controllers_netids: &mut HashMap<u32, u32>,
    socket: &UdpSocket,
    poll: &mut Poll,
    events: &mut Events,
    server_address: SocketAddr,
) -> Result<(), io::Error> {
    let mut buf = [0; 1 << 16];

    let serialized = to_vec::<IolEvent, 32>(&IolEvent::PhysicalDeviceAdded {
        which: controller.instance_id(),
    })
    .unwrap();
    let result = socket.send_to(serialized.as_slice(), server_address);
    match result {
        Ok(_) => {}
        Err(e) => {
            return Err(e);
        }
    }
    loop {
        if let Err(err) = poll.poll(events, Some(Duration::from_secs(5))) {
            match err.kind() {
                io::ErrorKind::Interrupted => {
                    continue;
                }
                _ => {
                    return Err(err);
                }
            }
        }
        match socket.recv_from(&mut buf) {
            Ok((packet_size, _)) => {
                let event = from_bytes::<IolEvent>(&buf[..packet_size]).unwrap();

                match event {
                    IolEvent::VirtualDeviceAdded { id, which } => {
                        println!("Controller {} was added on the listener.", id);
                        controllers_netids.insert(id, which);
                        return Ok(());
                    }
                    _ => {}
                }
            }
            Err(e) => {
                return Err(e);
            }
        }
    }
}

fn main() -> io::Result<()> {
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

    //    let mut buf = [0; 1 << 16];

    let mut broadcast_keyboard = true;
    let mut broadcast_gamepad = true;
    let mut server_address_str = "192.168.1.12:4863".to_owned();
    let mut server_address: SocketAddr =
        SocketAddr::new(IpAddr::V4(Ipv4Addr::new(192, 168, 1, 12)), 4863);

    let mut connected: bool = false;

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
    let mut controllers: Vec<GameController> = vec![];
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
                    if broadcast_keyboard {
                        let serialized = to_vec::<IolEvent, 32>(&IolEvent::KeyUp {
                            scancode: scancode.unwrap(),
                        })
                        .unwrap();

                        socket.send_to(serialized.as_slice(), server_address)?;
                    }
                }
                Event::ControllerDeviceAdded { which, .. } => {
                    println!("Controller {} was added.", which);

                    match controller_subsystem.open(which) {
                        Ok(c) => {
                            controllers.push(c);
                            if connected {
                                setup_controller_id(
                                    &controllers
                                        .iter()
                                        .find(|&c| c.instance_id() == which)
                                        .unwrap(),
                                    &mut controllers_netids,
                                    &socket,
                                    &mut poll,
                                    &mut events,
                                    server_address,
                                )
                                .expect("Unable to setup controller.");
                            }
                        }
                        Err(e) => {
                            println!("failed: {:?}", e);
                        }
                    }
                }

                Event::ControllerDeviceRemoved { which, .. } => {
                    let id = controllers_netids.get(&which).cloned();
                    if let Some(id) = id {
                        let serialized =
                            to_vec::<IolEvent, 32>(&IolEvent::PhysicalDeviceRemoved { id })
                                .unwrap();
                        socket.send_to(serialized.as_slice(), server_address)?;
                        controllers_netids.remove(&id);
                    }

                    controllers.remove(
                        controllers
                            .iter()
                            .position(|c| c.instance_id() == which)
                            .unwrap(),
                    );
                    println!("Controller {} was removed.", which);
                }
                Event::ControllerButtonDown { which, button, .. } => {
                    let id = controllers_netids.get(&which);
                    if let Some(id) = id {
                        let serialized =
                            to_vec::<IolEvent, 32>(&IolEvent::ButtonDown { id: *id, button })
                                .unwrap();
                        socket.send_to(serialized.as_slice(), server_address)?;
                    }
                }
                Event::ControllerButtonUp { which, button, .. } => {
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
                    let id = controllers_netids.get(&which);
                    let fixed_value = match axis {
                        sdl2::controller::Axis::LeftY | sdl2::controller::Axis::RightX => {
                            if value == -32768 {
                                (value + 1) * -1
                            } else {
                                value * -1
                            }
                        }
                        _ => value,
                    };
                    if let Some(id) = id {
                        let serialized = to_vec::<IolEvent, 32>(&IolEvent::AxisMotion {
                            id: *id,
                            axis,
                            value: fixed_value,
                        })
                        .unwrap();
                        socket.send_to(serialized.as_slice(), server_address)?;
                    }
                }
                _ => {}
            }
        }

        platform.prepare_frame(&mut imgui, &window, &event_pump);
        let ui = imgui.new_frame();

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
                if !connected {
                    if ui.button("Connect") {
                        server_address = server_address_str
                            .parse()
                            .expect("Unable to parse socket address");
                        for controller in controllers.iter() {
                            let result = setup_controller_id(
                                controller,
                                &mut controllers_netids,
                                &socket,
                                &mut poll,
                                &mut events,
                                server_address,
                            );
                            match result {
                                Ok(()) => {
                                    connected = true;
                                }
                                Err(e) => match e.kind() {
                                    _ => {
                                        println!("Unable to setup controller. {:#?}", e)
                                    }
                                },
                            }
                        }
                    }
                } else {
                    if ui.button("Disconnect") {
                        for controller in controllers_netids.iter() {
                            let id = *controller.0;
                            let serialized =
                                to_vec::<IolEvent, 32>(&IolEvent::PhysicalDeviceRemoved { id })
                                    .unwrap();
                            socket.send_to(serialized.as_slice(), server_address).ok();
                            println!("Controller {} was removed on the listener.", id);
                        }
                        controllers_netids.clear();
                        connected = false;
                    }
                }
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
                    let controller = controller_iter.next().unwrap();
                    ui.text(controller.name());
                    ui.same_line();
                    let id = controllers_netids.get(&controller.instance_id());
                    let id_string = if let Some(id) = id {
                        format!("id: {}", id)
                    } else {
                        "id: n/a".to_string()
                    };

                    ui.text_colored([1.0, 1.0, 0.0, 1.0], id_string);
                }
            });

        let draw_data = imgui.render();

        unsafe { renderer.gl_context().clear(glow::COLOR_BUFFER_BIT) };
        renderer.render(draw_data).unwrap();

        window.gl_swap_window();
    }

    Ok(())
}
