use sdl2::{
    self, Sdl, VideoSubsystem,
    pixels::PixelFormatEnum,
    render::{Texture, TextureCreator, WindowCanvas},
    video::{Window, WindowContext},
};
use std::{
    cell::{OnceCell, RefCell},
    sync::LazyLock,
    time::{Duration, Instant},
};

struct WindowState {
    canvas: WindowCanvas,
    texture_creator: TextureCreator<WindowContext>,
    texture: Texture,
}

static START_TIME: LazyLock<Instant> = LazyLock::new(|| Instant::now());

thread_local! {
    static SDL_CONTEXT: OnceCell<Sdl> = OnceCell::new();
    static SDL_VIDEO_SUBSYSTEM: OnceCell<VideoSubsystem> = OnceCell::new();
    static SDL_WINDOW: RefCell<Option<WindowState>> = RefCell::new(None);
}

pub fn use_sdl_context<T, F: FnOnce(&Sdl) -> T>(f: F) -> T {
    SDL_CONTEXT.with(|cell| {
        let ctx = cell.get_or_init(|| sdl2::init().expect("failed to intialize sdl2 context"));
        f(ctx)
    })
}

pub fn use_video_subsytem<T, F: FnOnce(&VideoSubsystem) -> T>(f: F) -> T {
    use_sdl_context(|ctx| {
        SDL_VIDEO_SUBSYSTEM.with(|cell| {
            let vs =
                cell.get_or_init(|| ctx.video().expect("failed to initialize video subsystem"));
            f(vs)
        })
    })
}

pub fn create_window(title: &str, width: u32, height: u32) {
    use_video_subsytem(|vs| {
        let canvas = vs
            .window(title, width, height)
            .build()
            .expect("failed to create window")
            .into_canvas()
            .accelerated()
            .present_vsync()
            .build()
            .expect("failed to create canvas");
        let texture_creator = canvas.texture_creator();
        let texture = texture_creator
            .create_texture_target(PixelFormatEnum::RGB888, width, height)
            .expect("failed to create texture");
        SDL_WINDOW.set(Some(WindowState {
            canvas,
            texture_creator,
            texture,
        }));
    })
}

pub fn use_window_mut<T, F: FnOnce(&mut Window) -> T>(f: F) -> T {
    SDL_WINDOW.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let state = borrow.as_mut().expect("no window!");
        f(state.canvas.window_mut())
    })
}

pub fn delay_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms))
}

pub fn get_ticks_ms() -> u64 {
    START_TIME.elapsed().as_millis() as u64
}

pub fn render(pixels: &[u8], width: u32) {
    SDL_WINDOW.with_borrow_mut(|state| {
        if let Some(WindowState {
            canvas,
            texture_creator: _,
            texture,
        }) = state
        {
            texture
                .update(None, pixels, width as usize * 4)
                .expect("failed to update texture");
            canvas.clear();
            canvas
                .copy(texture, None, None)
                .expect("failed to copy texture to canvas");
            canvas.present();
        }
    })
}
