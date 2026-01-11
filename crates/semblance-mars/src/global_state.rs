use sdl2::{self, Sdl, VideoSubsystem, video::Window};
use std::cell::{OnceCell, RefCell};

thread_local! {
    static SDL_CONTEXT: OnceCell<Sdl> = OnceCell::new();
    static SDL_VIDEO_SUBSYSTEM: OnceCell<VideoSubsystem> = OnceCell::new();
    static SDL_WINDOW: RefCell<Option<Window>> = RefCell::new(None);
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
        SDL_WINDOW.set(Some(
            vs.window(title, width, height)
                .build()
                .expect("failed to create window"),
        ))
    })
}

pub fn use_window_mut<T, F: FnOnce(&mut Window) -> T>(f: F) -> T {
    SDL_WINDOW.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let window_mut = borrow.as_mut().expect("no window!");
        f(window_mut)
    })
}
