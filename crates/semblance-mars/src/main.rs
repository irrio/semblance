use sdl2::{event::Event, keyboard::Keycode};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem
        .window("Semblance Mars", 800, 480)
        .position_centered()
        .build()?;

    println!("Opened SDL2 Window: {}", window.title());

    let mut event_pump = sdl_context.event_pump()?;

    'running: loop {
        for event in event_pump.poll_iter() {
            match event {
                Event::Quit { .. }
                | Event::KeyUp {
                    keycode: Some(Keycode::ESCAPE),
                    ..
                } => {
                    break 'running;
                }
                _ => (),
            }
        }
    }

    Ok(())
}
