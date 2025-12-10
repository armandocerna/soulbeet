use dioxus::prelude::*;

const DOWNLOAD_FULL_ALBUMS_KEY: &str = "soulbeet_download_full_albums";

fn get_local_storage() -> Option<web_sys::Storage> {
    web_sys::window()?.local_storage().ok()?
}

pub fn get_download_full_albums() -> bool {
    get_local_storage()
        .and_then(|storage| storage.get_item(DOWNLOAD_FULL_ALBUMS_KEY).ok())
        .flatten()
        .map(|v| v == "true")
        .unwrap_or(false)
}

pub fn set_download_full_albums(enabled: bool) {
    if let Some(storage) = get_local_storage() {
        let _ = storage.set_item(DOWNLOAD_FULL_ALBUMS_KEY, if enabled { "true" } else { "false" });
    }
}

#[derive(Clone, Copy)]
pub struct DownloadFullAlbumsPreference {
    signal: Signal<bool>,
}

impl DownloadFullAlbumsPreference {
    pub fn get(&self) -> bool {
        *self.signal.read()
    }

    pub fn set(&mut self, enabled: bool) {
        set_download_full_albums(enabled);
        self.signal.set(enabled);
    }

    pub fn toggle(&mut self) {
        let new_value = !self.get();
        self.set(new_value);
    }
}

pub fn use_download_full_albums() -> DownloadFullAlbumsPreference {
    let signal = use_signal(get_download_full_albums);
    DownloadFullAlbumsPreference { signal }
}
