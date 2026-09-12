use headless_chrome::{Browser, LaunchOptions};
use std::sync::{Mutex, OnceLock};

fn browser_state() -> &'static Mutex<Option<(Browser, std::sync::Arc<headless_chrome::Tab>)>> {
    static STATE: OnceLock<Mutex<Option<(Browser, std::sync::Arc<headless_chrome::Tab>)>>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(None))
}

fn main() {
    let options = headless_chrome::LaunchOptions::default_builder()
        .headless(false)
        .build()
        .unwrap();
    let browser = Browser::new(options).unwrap();
    let tab = browser.new_tab().unwrap();
    
    let mut state = browser_state().lock().unwrap();
    *state = Some((browser, tab));
}
