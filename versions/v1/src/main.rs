//https://docs.rs/wayland-client/latest/wayland_client/
//https://github.com/Smithay/wayland-rs/blob/master/wayland-client/examples/simple_window.rs

use std::{fs::File, os::unix::io::AsFd};

use wayland_client::{
    delegate_noop,
    protocol::{
    wl_registry, wl_compositor,
    wl_buffer, wl_keyboard, wl_pointer, wl_seat, 
    wl_shm, wl_shm_pool,
    wl_surface,
}, Connection, Dispatch, QueueHandle, WEnum,
};

use wayland_protocols::xdg::shell::client::{xdg_surface, xdg_toplevel, xdg_wm_base};

// This struct represents the state of our app. This simple app does not
// need any state, but this type still supports the `Dispatch` implementations.
struct State {
    running: bool,
    base_surface: Option<wl_surface::WlSurface>,
    buffer: Option<wl_buffer::WlBuffer>,
    wm_base: Option<xdg_wm_base::XdgWmBase>,
    xdg_surface: Option<(xdg_surface::XdgSurface, xdg_toplevel::XdgToplevel)>,
    configured: bool,
    seat: Option<wl_seat::WlSeat>,
    x_cord: f64,
    y_cord: f64,
    xi_cord: i32,
    yi_cord: i32,
    width: u32,
    height: u32,
    n_width: u32,
    n_height: u32,
    select_square_diameter: f64,
    pointer_serial: u32,
    pointer_surface: Option<wl_surface::WlSurface>,
    //registry: &wl_registry::WlRegistry,
    shm: Option<wl_shm::WlShm>,
    resize_ready: bool
}
// The main function of our program
fn main() {
    // Create a Wayland connection by connecting to the server through the
    // environment-provided configuration.
    let conn = Connection::connect_to_env().unwrap();

    // Retrieve the WlDisplay Wayland object from the connection. This object is
    // the starting point of any Wayland program, from which all other objects will
    // be created.
    let display = conn.display();

    // Create an event queue for our event processing
    let mut event_queue = conn.new_event_queue();
    // And get its handle to associate new objects to it
    let qhandler = event_queue.handle();

    // Create a wl_registry object by sending the wl_display.get_registry request.
    // This method takes two arguments: a handle to the queue that the newly created
    // wl_registry will be assigned to, and the user-data that should be associated
    // with this registry (here it is () as we don't need user-data).
    let _registry = display.get_registry(&qhandler, ());

    // At this point everything is ready, and we just need to wait to receive the events
    // from the wl_registry. Our callback will print the advertised globals.
    println!("Advertised globals:");

    // To actually receive the events, we invoke the `roundtrip` method. This method
    // is special and you will generally only invoke it during the setup of your program:
    // it will block until the server has received and processed all the messages you've
    // sent up to now.
    //
    // In our case, that means it'll block until the server has received our
    // wl_display.get_registry request, and as a reaction has sent us a batch of
    // wl_registry.global events.
    //
    // `roundtrip` will then empty the internal buffer of the queue it has been invoked
    // on, and thus invoke our `Dispatch` implementation that prints the list of advertised
    // globals.
    //event_queue.roundtrip(&mut AppData).unwrap();

    let mut state = State {
        running: true,
        base_surface: None,
        buffer: None,
        wm_base: None,
        xdg_surface: None,
        configured: false,
        seat: None,
        x_cord: 0.0,
        y_cord: 0.0,
        xi_cord: 0,
        yi_cord: 0,
        width: 640,//320,
        height: 480,//240
        n_width: 640,
        n_height: 480,
        select_square_diameter: 40.0,
        pointer_serial: 0,
        pointer_surface: None,
        shm: None,
        resize_ready: true
    };

    println!("Starting the example window app, press <ESC> to quit.");

    while state.running {
        event_queue.blocking_dispatch(&mut state).unwrap();
    }
}

impl State {
    fn init_xdg_surface(&mut self, qh: &QueueHandle<State>) {
        let wm_base = self.wm_base.as_ref().unwrap();
        let base_surface = self.base_surface.as_ref().unwrap();

        let xdg_surface = wm_base.get_xdg_surface(base_surface, qh, ());
        let toplevel = xdg_surface.get_toplevel(qh, ());
        toplevel.set_title("A fantastic window!".into());


        base_surface.commit();

        self.xdg_surface = Some((xdg_surface, toplevel));
    }

    fn shm_this(&mut self, qh: &QueueHandle<State>) {
        print!("SHM func started!\n");

        let (init_w, init_h) = (self.width, self.height);//(320, 240);

        let mut file = tempfile::tempfile().unwrap();
        draw(&mut file, (init_w, init_h), self);
        let pool = self.shm.as_ref().unwrap().create_pool(file.as_fd(), (init_w * init_h * 4) as i32, qh, ());
        let buffer = pool.create_buffer(
            0,
            init_w as i32,
            init_h as i32,
            (init_w * 4) as i32,
            wl_shm::Format::Argb8888,
            qh,
            (),
        );
        self.buffer = Some(buffer.clone());

        if self.configured {
            let surface = self.base_surface.as_ref().unwrap();
            surface.attach(Some(&buffer), 0, 0);
            surface.commit();
        }
    }
}

// Implement `Dispatch<WlRegistry, ()> for our state. This provides the logic
// to be able to process events for the wl_registry interface.
//
// The second type parameter is the user-data of our implementation. It is a
// mechanism that allows you to associate a value to each particular Wayland
// object, and allow different dispatching logic depending on the type of the
// associated value.
//
// In this example, we just use () as we don't have any value to associate. See
// the `Dispatch` documentation for more details about this.
impl Dispatch<wl_registry::WlRegistry, ()> for State {
    fn event(
        state: &mut Self,
        registry: &wl_registry::WlRegistry,
        event: wl_registry::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        // When receiving events from the wl_registry, we are only interested in the
        // `global` event, which signals a new available global.
        // When receiving this event, we just print its characteristics in this example.
        if let wl_registry::Event::Global { name, interface, .. } = event {
            println!("[{}] {} (v)", name, interface);
            println!(": {} :", &interface[..]);
            match &interface[..] {
                "wl_compositor" => {
                    let compositor =
                        registry.bind::<wl_compositor::WlCompositor, _, _>(name, 1, qh, ());
                    let surface = compositor.create_surface(qh, ());
                    state.base_surface = Some(surface);

                    if state.wm_base.is_some() && state.xdg_surface.is_none() {
                        state.init_xdg_surface(qh);
                    }
                }
                "wl_shm" => {
                    print!("SHM started!\n");
                    let shm = registry.bind::<wl_shm::WlShm, _, _>(name, 1, qh, ());

                    state.shm = Some(shm.clone());

                    let (init_w, init_h) = (state.width, state.height);//(320, 240);

                    let mut file = tempfile::tempfile().unwrap();
                    draw(&mut file, (init_w, init_h), state);
                    let pool = shm.create_pool(file.as_fd(), (init_w * init_h * 4) as i32, qh, ());
                    let buffer = pool.create_buffer(
                        0,
                        init_w as i32,
                        init_h as i32,
                        (init_w * 4) as i32,
                        wl_shm::Format::Argb8888,
                        qh,
                        (),
                    );
                    state.buffer = Some(buffer.clone());

                    if state.configured {
                        let surface = state.base_surface.as_ref().unwrap();
                        surface.attach(Some(&buffer), 0, 0);
                        surface.commit();
                    }
                }
                "wl_seat" => {
                    registry.bind::<wl_seat::WlSeat, _, _>(name, 1, qh, ());
                }
                "xdg_wm_base" => {
                    let wm_base = registry.bind::<xdg_wm_base::XdgWmBase, _, _>(name, 1, qh, ());
                    state.wm_base = Some(wm_base);

                    if state.base_surface.is_some() && state.xdg_surface.is_none() {
                        state.init_xdg_surface(qh);
                    }
                }
                _ => {}
            }
        }
    }
}

// Ignore events from these object types in this example.
delegate_noop!(State: ignore wl_compositor::WlCompositor);
delegate_noop!(State: ignore wl_surface::WlSurface);
delegate_noop!(State: ignore wl_shm::WlShm);
delegate_noop!(State: ignore wl_shm_pool::WlShmPool);
delegate_noop!(State: ignore wl_buffer::WlBuffer);

fn draw(tmp: &mut File, (buf_x, buf_y): (u32, u32), state: &mut State) {
    use std::{cmp::min, io::Write};
    let mut buf = std::io::BufWriter::new(tmp);
    for y in 0..buf_y {
        for x in 0..buf_x {
            let a = 0xFF;
            let r: u32;
            let g: u32;
            let b: u32;
            if x <= state.select_square_diameter as u32 && y <= state.select_square_diameter as u32 {
                //let a =
                r = 0;
                g = 0;
                b = 0;
            //} else if x >= state.select_square_diameter as u32 && y >= state.select_square_diameter as u32 {
            } else if buf_x - x <= state.select_square_diameter as u32 && buf_y - y <= state.select_square_diameter as u32 {
                r = 0xFF;
                g = 0xFF;
                b = 0xFF;
            } else {
                r = min(((buf_x - x) * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
                g = min((x * 0xFF) / buf_x, ((buf_y - y) * 0xFF) / buf_y);
                b = min(((buf_x - x) * 0xFF) / buf_x, (y * 0xFF) / buf_y);
            }
            buf.write_all(&[b as u8, g as u8, r as u8, a as u8]).unwrap();
        }
    }
    buf.flush().unwrap();
}


impl Dispatch<xdg_wm_base::XdgWmBase, ()> for State {
    fn event(
        _: &mut Self,
        wm_base: &xdg_wm_base::XdgWmBase,
        event: xdg_wm_base::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            wm_base.pong(serial);
        }
    }
}

impl Dispatch<xdg_surface::XdgSurface, ()> for State {
    fn event(
        state: &mut Self,
        xdg_surface: &xdg_surface::XdgSurface,
        event: xdg_surface::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial, .. } = event {
            if (state.n_height != state.height || state.n_width != state.width) && state.resize_ready == true {
                print!("Configure changes screen size\n\n");
                state.height = state.n_height;
                state.width = state.n_width;
                state.shm_this(qh);
            }
            println!("Configure: {:?}\n\n {} {}", state.xdg_surface.as_ref().unwrap().1, state.resize_ready, state.n_height != state.height || state.n_width != state.width);
            xdg_surface.ack_configure(serial);
            state.configured = true;
            let surface = state.base_surface.as_ref().unwrap();
            if let Some(ref buffer) = state.buffer {
                surface.attach(Some(buffer), 0, 0);
                surface.commit();
            }
        }
    }
}

impl Dispatch<xdg_toplevel::XdgToplevel, ()> for State {
    fn event(
        state: &mut Self,
        _xdg_toplevel: &xdg_toplevel::XdgToplevel,
        event: xdg_toplevel::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_toplevel::Event::Close {} = event {
            state.running = false;
        }

        if let xdg_toplevel::Event::Configure { width, height, ref states, .. } = event {
            println!("New Width: {}\nNew Height: {}\nStates: {:?}", width, height, states);
            if height != 0 && width != 0 {
                println!("{}", *states == []);
                if *states == [] || states[0] != 3 || state.resize_ready {// || ((state.n_height == height as u32 && state.n_width == width as u32)) {
                    state.resize_ready = true;
                    print!("\n\nShould fix this shit now\n\n");
                    if *states != [] && states[0] != 3 {
                        state.n_height = height as u32;
                        state.n_width = width as u32;
                    }
                } else {
                    state.n_height = height as u32;
                    state.n_width = width as u32;
                }
            }
        }

        if let xdg_toplevel::Event::ConfigureBounds { width, height, .. } = event {
            println!("{} {}", width, height);
        }
    }
}

impl Dispatch<wl_seat::WlSeat, ()> for State {
    fn event(
        state: &mut Self,
        seat: &wl_seat::WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        qh: &QueueHandle<Self>,
    ) {
        print!("Seat dispatch");
        if let wl_seat::Event::Capabilities { capabilities: WEnum::Value(capabilities) } = event {
            if capabilities.contains(wl_seat::Capability::Keyboard) {
                seat.get_keyboard(qh, ());
            }
            if capabilities.contains(wl_seat::Capability::Pointer) {
                seat.get_pointer(qh, ());
            }
        }
        state.seat = Some(seat.clone());
    }
}

impl Dispatch<wl_keyboard::WlKeyboard, ()> for State {
    fn event(
        state: &mut Self,
        _: &wl_keyboard::WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_keyboard::Event::Key { key, .. } = event {
            println!("Key pressed: {}", key);
            if key == 1 {
                // ESC key
                state.running = false;
            } 
            else if key == 12 || key == 53 {
                println!("+ or - pressed!, code: {}", key);
            }  
        }
    }
}

impl Dispatch<wl_pointer::WlPointer, ()> for State {
    fn event(
        _state: &mut Self,
        _pointer: &wl_pointer::WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _qh: &QueueHandle<Self>,
    ) {
        if let wl_pointer::Event::Button { serial, button, state, .. } = event {
            println!("Pointer pressed: {}, with a state of {:?}, u8: {}", button, state, u32::from(state));
            if button == 273 {
                _state.xdg_surface.as_ref().unwrap().1.show_window_menu(&_state.seat.as_ref().unwrap(), serial, _state.xi_cord, _state.yi_cord);
            }
            if button == 272 {
                println!("{}", u32::from(state) == 1);
                if u32::from(state) == 1 {
                    if _state.select_square_diameter >= _state.x_cord && _state.select_square_diameter >= _state.y_cord {
                        _state.xdg_surface.as_ref().unwrap().1._move(&_state.seat.as_ref().unwrap(), serial);
                    } else if _state.width - _state.xi_cord as u32 <= _state.select_square_diameter as u32 && _state.height - _state.yi_cord as u32 <= _state.select_square_diameter as u32 {
                        _state.resize_ready = false;
                        _state.xdg_surface.as_ref().unwrap().1.resize(&_state.seat.as_ref().unwrap(), serial, xdg_toplevel::ResizeEdge::BottomRight);
                    }
                } else {
                    _state.resize_ready = true;
                }
            }
        }
        if let wl_pointer::Event::Enter { serial, ref surface, surface_x, surface_y, .. } = event {
            println!("Pointer entered: {}, on surface {:?} at X: {} Y: {}", serial, surface, surface_x, surface_y);
            _state.xdg_surface.as_ref().unwrap().1._move(&_state.seat.as_ref().unwrap(), serial);
            _state.x_cord = surface_x;
            _state.y_cord = surface_y;
            _state.xi_cord = surface_x as i32;
            _state.yi_cord = surface_y as i32;

            _state.pointer_serial = serial;
            _state.pointer_surface = Some(surface.clone());
        }

        if let wl_pointer::Event::Motion { surface_x, surface_y, .. } = event {
            _state.x_cord = surface_x;
            _state.y_cord = surface_y;
            _state.xi_cord = surface_x as i32;
            _state.yi_cord = surface_y as i32;
        }
    }
}
