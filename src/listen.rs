use log::warn;
use mio::{Events, Interest, Poll, Token};
use std::{
    io,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

const UDP_SOCKET: Token = Token(0);
const PORT: u16 = 4863;

fn main() -> io::Result<()> {
    use std::{collections::HashMap, rc::Rc};

    use iol::{vigem::ViGEMState, IolEvent};
    use mio::net::UdpSocket;
    use postcard::{from_bytes, to_vec};
    use vigem_client::TargetId;

    env_logger::init();

    let mut poll = Poll::new()?;
    let mut events = Events::with_capacity(1);
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), PORT);

    let mut socket = UdpSocket::bind(addr)?;

    poll.registry().register(
        &mut socket,
        UDP_SOCKET,
        Interest::WRITABLE | Interest::READABLE,
    )?;

    println!("You can connect to the server via port {}", PORT);

    let mut buf = [0; 1 << 16];
    let mut controllers: HashMap<u32, ViGEMState> = HashMap::new();
    let vigem_client = Rc::new(vigem_client::Client::connect().unwrap());

    loop {
        if let Err(err) = poll.poll(&mut events, None) {
            if err.kind() == io::ErrorKind::Interrupted {
                continue;
            }
            return Err(err);
        }

        for event in events.iter() {
            match event.token() {
                UDP_SOCKET => loop {
                    match socket.recv_from(&mut buf) {
                        Ok((packet_size, source_address)) => {
                            let event = from_bytes::<IolEvent>(&buf[..packet_size]);
                            match event.unwrap() {
                                // TODO: Keyboard emulation
                                IolEvent::KeyDown { .. } => {}
                                IolEvent::KeyUp { .. } => {}
                                IolEvent::PhysicalDeviceAdded { which } => {
                                    let id = controllers.len() as u32;
                                    println!("Controller {} was added.", id);
                                    let mut target = vigem_client::Xbox360Wired::new(
                                        vigem_client.clone(),
                                        TargetId::XBOX360_WIRED,
                                    );
                                    target.plugin().unwrap();
                                    target.wait_ready().unwrap();
                                    controllers.insert(id, ViGEMState::new(vigem_client.clone()));

                                    let serialized =
                                        to_vec::<IolEvent, 32>(&IolEvent::VirtualDeviceAdded {
                                            id,
                                            which,
                                        })
                                        .unwrap();

                                    socket.send_to(serialized.as_slice(), source_address)?;
                                    println!("Controller virtual device {} was added.", id)
                                }
                                IolEvent::PhysicalDeviceRemoved { id } => {
                                    controllers.remove(&id);
                                    println!("Controller {} was removed.", id);
                                }
                                IolEvent::ButtonDown { id, button } => {
                                    let controller = controllers.get_mut(&id);
                                    if let Some(controller) = controller {
                                        controller.from_sdl2_button(button, true)
                                    }
                                }
                                IolEvent::ButtonUp { id, button } => {
                                    let controller = controllers.get_mut(&id);
                                    if let Some(controller) = controller {
                                        controller.from_sdl2_button(button, false);
                                    }
                                }
                                IolEvent::AxisMotion {
                                    id, axis, value, ..
                                } => {
                                    let controller = controllers.get_mut(&id);
                                    if let Some(controller) = controller {
                                        controller.from_sdl2_axis(axis, value);
                                    }
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
                    warn!("Got event for unexpected token: {:?}", event);
                }
            }
        }
    }
}
