use log::warn;
use mio::{Events, Interest, Poll, Token};
use std::io;

// A token to allow us to identify which event is for the `UdpSocket`.
const UDP_SOCKET: Token = Token(0);

#[cfg(target_os = "windows")]
fn main() -> io::Result<()> {
    use std::collections::HashMap;

    use iol::IolEvent;
    use mio::net::UdpSocket;
    use postcard::{from_bytes, to_vec};

    env_logger::init();

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1);
    let addr = "0.0.0.0:4863".parse().unwrap();

    let mut socket = UdpSocket::bind(addr)?;

    // Register our socket with the token defined above and an interest in being
    // `READABLE`.
    poll.registry().register(
        &mut socket,
        UDP_SOCKET,
        Interest::WRITABLE | Interest::READABLE,
    )?;

    println!("You can connect to the server via port 4863");

    let mut buf = [0; 1 << 16];
    let mut controllers: HashMap<u32, GamepadState> = HashMap::new();
    let vigem_client = vigem_client::Client::connect().unwrap();

    loop {
        if let Err(err) = poll.poll(&mut events, None) {
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err);
        }

        // Process each event.
        for event in events.iter() {
            // Validate the token we registered our socket with,
            // in this example it will only ever be one but we
            // make sure it's valid none the less.
            match event.token() {
                UDP_SOCKET => loop {
                    match socket.recv_from(&mut buf) {
                        Ok((packet_size, source_address)) => {
                            let event = from_bytes::<IolEvent>(&buf[..packet_size]);
                            match event.unwrap() {
                                IolEvent::KeyDown { scancode, repeat } => {
                                    if !repeat {
                                        println!("Scancode {:?} was pressed.", scancode)
                                    };
                                }
                                IolEvent::KeyUp { scancode } => {
                                    println!("Scancode {:?} was released.", scancode);
                                }
                                IolEvent::PhysicalDeviceAdded { which } => {
                                    //TODO Setup controller device added
                                    let id = controllers.len() as u32;
                                    println!("Controller with index {} was added.", id);

                                    controllers.insert(id, 0);
                                    let mut target = vigem_client::Xbox360Wired::new(client, id);

                                    let serialized =
                                        to_vec::<IolEvent, 32>(&IolEvent::VirtualDeviceAdded {
                                            id,
                                            which,
                                        })
                                        .unwrap();

                                    socket.send_to(serialized.as_slice(), source_address)?;
                                    println!(
                                        "Controller virtual device with index {} was added.",
                                        id
                                    )
                                }
                                IolEvent::PhysicalDeviceRemoved { id } => {
                                    controllers.remove(&id);
                                    println!("Controller with index {} was removed.", id);
                                }
                                IolEvent::ButtonDown { id, button } => {
                                    println!("Controller with index {} pressed {:?}.", id, button);
                                }
                                IolEvent::ButtonUp { id, button } => {
                                    println!("Controller with index {} released {:?}.", id, button);
                                }
                                IolEvent::AxisMotion {
                                    id, axis, value, ..
                                } => {
                                    println!(
                                        "Controller with index {} moved axis {:?} to {:?}.",
                                        id, axis, value
                                    );
                                }
                                _ => {}
                            }
                        }
                        Err(e) if e.kind() == io::ErrorKind::WouldBlock => {
                            break;
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                },
                _ => {
                    // This should never happen as we only registered our
                    // `UdpSocket` using the `UDP_SOCKET` token, but if it ever
                    // does we'll log it.
                    warn!("Got event for unexpected token: {:?}", event);
                }
            }
        }
    }
}
