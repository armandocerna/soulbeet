pub mod album;
pub mod context;
pub mod track;

pub use context::SearchReset;

use dioxus::logger::tracing::info;
use dioxus::prelude::*;
use shared::download::DownloadQuery;
use shared::musicbrainz::{AlbumWithTracks, SearchResult};
use shared::slskd::{
    AlbumResult as SlskdAlbumResult, SearchState, TrackResult as SlskdTrackResult,
};

use track::TrackResult;

use crate::search::album::AlbumResult;
use crate::{use_auth, use_download_full_albums, Album, AlbumHeader, Button, Modal};

mod download_results;
use download_results::DownloadResults;

#[component]
pub fn Search() -> Element {
    let auth = use_auth();
    let mut response = use_signal::<Option<Vec<SearchResult>>>(|| None);
    let mut search = use_signal(String::new);
    let mut artist = use_signal::<Option<String>>(|| None);
    let mut loading = use_signal(|| false);
    let mut viewing_album = use_signal::<Option<AlbumWithTracks>>(|| None);
    let mut download_options = use_signal::<Option<Vec<SlskdAlbumResult>>>(|| None);
    let search_reset = try_use_context::<SearchReset>();
    let mut download_full_albums = use_download_full_albums();

    use_effect(move || {
        if let Some(reset) = search_reset {
            if reset.0() > 0 {
                response.set(None);
                search.set(String::new());
                artist.set(None);
                viewing_album.set(None);
                download_options.set(None);
            }
        }
    });

    if !auth.is_logged_in() {
        info!("User not logged in");
        return rsx! {};
    }

    let download = move |query: DownloadQuery| async move {
        loading.set(true);
        viewing_album.set(None);
        download_options.set(Some(vec![]));

        let search_id = match auth.call(api::start_download_search(query)).await {
            Ok(id) => id,
            Err(e) => {
                warn!("Failed to start download search: {:?}", e);
                loading.set(false);
                return;
            }
        };

        loop {
            match auth
                .call(api::poll_download_search(search_id.clone()))
                .await
            {
                Ok(response) => {
                    download_options.with_mut(|current| {
                        if let Some(list) = current {
                            for new_album in response.results {
                                if let Some(pos) = list.iter().position(|x| {
                                    x.username == new_album.username
                                        && x.album_path == new_album.album_path
                                }) {
                                    // Safeguard against incomplete albums
                                    list[pos] = new_album;
                                } else {
                                    list.push(new_album);
                                }
                            }

                            // Resort new results by score
                            list.sort_by(|a, b| {
                                b.score
                                    .partial_cmp(&a.score)
                                    .unwrap_or(std::cmp::Ordering::Equal)
                            });
                        }
                    });

                    if response.state != SearchState::InProgress {
                        break;
                    }
                }
                Err(e) => {
                    info!("Failed to poll search: {:?}", e);
                    break;
                }
            }
        }
        loading.set(false);
    };

    let download_tracks = move |(tracks, folder): (Vec<SlskdTrackResult>, String)| async move {
        loading.set(true);
        download_options.set(None);
        if let Ok(_res) = auth.call(api::download(tracks, folder)).await {
            info!("Downloads started");
        }
        loading.set(false);
    };

    let search_track = move || async move {
        loading.set(true);

        if let Ok(data) = auth
            .call(api::search_track(api::SearchQuery {
                artist: artist(),
                query: search(),
            }))
            .await
        {
            response.set(Some(data));
        }
        loading.set(false);
    };

    let search_album = move || async move {
        loading.set(true);

        if let Ok(data) = auth
            .call(api::search_album(api::SearchQuery {
                artist: artist(),
                query: search(),
            }))
            .await
        {
            response.set(Some(data));
        }
        loading.set(false);
    };

    let browse_artist = move || async move {
        loading.set(true);

        if let Some(artist_name) = artist() {
            if let Ok(data) = auth
                .call(api::browse_artist(api::BrowseArtistQuery {
                    artist: artist_name,
                }))
                .await
            {
                response.set(Some(data));
            }
        }
        loading.set(false);
    };

    let view_full_album = move |album_id: String| async move {
        loading.set(true);

        if let Ok(album_data) = auth.call(api::find_album(album_id.clone())).await {
            if download_full_albums.get() {
                // Skip the track selection modal and immediately start download with all tracks
                let query = DownloadQuery {
                    album: album_data.album,
                    tracks: album_data.tracks,
                };
                // Reset loading since download() will set it again
                loading.set(false);
                spawn(download(query));
            } else {
                viewing_album.set(Some(album_data));
                loading.set(false);
            }
        } else {
            info!("Failed to fetch album details for {}", album_id);
            loading.set(false);
        }
    };

    if let Some(results) = download_options.read().clone() {
        return rsx! {
          DownloadResults {
            results,
            is_searching: loading(),
            on_download: move |data| {
                spawn(download_tracks(data));
            },
          }
        };
    }

    rsx! {
      if let Some(data) = viewing_album.read().clone() {
        Modal {
          on_close: move |_| viewing_album.set(None),
          header: rsx! {
            AlbumHeader { album: data.album.clone() }
          },
          Album {
            data,
            on_select: move |data: DownloadQuery| {
                spawn(download(data));
            },
          }
        }
      }

      div { class: "bg-gray-800 text-white p-6 sm:p-8 rounded-lg shadow-xl max-w-2xl mx-auto my-10 font-sans",

        h4 { class: "text-2xl font-bold mb-6 text-center text-teal-400",
          "Search a track / album"
        }

        div { class: "flex flex-col sm:flex-row gap-4 mb-4",

          input {
            value: "{search}",
            class: "flex-grow bg-gray-700 text-white placeholder-gray-400 px-4 py-2 rounded-md border border-gray-600 focus:outline-none focus:ring-2 focus:ring-teal-500 transition-shadow",
            placeholder: "Search an album or track...",
            oninput: move |event| search.set(event.value()),
          }
          input {
            value: artist.read().as_ref().map_or("", |v| v),
            class: "flex-grow bg-gray-700 text-white placeholder-gray-400 px-4 py-2 rounded-md border border-gray-600 focus:outline-none focus:ring-2 focus:ring-teal-500 transition-shadow",
            placeholder: "Artist (optional)",
            oninput: move |event| {
                let input = event.value();
                if input.is_empty() {
                    artist.set(None);
                } else {
                    artist.set(Some(input));
                }
            },
          }
        }
        div { class: "flex justify-center gap-4 mb-4 flex-wrap",

          Button {
            disabled: loading() || search.read().is_empty(),
            onclick: move |_| search_track(),

            {"Search a Track"}
          }
          Button {
            disabled: loading() || search.read().is_empty(),
            onclick: move |_| search_album(),

            {"Search an Album"}
          }
          Button {
            disabled: loading() || artist.read().is_none(),
            onclick: move |_| browse_artist(),

            {"Browse Artist"}
          }
        }

        div { class: "flex justify-center items-center gap-2 mb-8",
          label { class: "flex items-center gap-2 cursor-pointer text-sm text-gray-300 hover:text-white transition-colors",
            input {
              r#type: "checkbox",
              checked: download_full_albums.get(),
              onchange: move |_| download_full_albums.toggle(),
              class: "w-4 h-4 rounded bg-gray-700 border-gray-600 text-teal-500 focus:ring-teal-500 focus:ring-offset-gray-800 cursor-pointer",
            }
            "Download full albums (skip track selection)"
          }
        }

        if loading() {
          div { class: "flex flex-col justify-center items-center py-10",
            div { class: "animate-spin rounded-full h-16 w-16 border-t-4 border-b-4 border-teal-500" }
          }
        } else {
          match *response.read() {
              Some(ref items) if !items.is_empty() => rsx! {
                h5 { class: "text-xl font-semibold mb-4 border-b border-gray-600 pb-2", "Results" }
                ul { class: "list-none p-0 space-y-4",
                  for item in items.iter() {
                    match item {
                        SearchResult::Track(ref track) => rsx! {
                          li { key: "{track.id}",
                            TrackResult { on_album_click: move |id| view_full_album(id), track: track.clone() }
                          }
                        },
                        SearchResult::Album(album) => rsx! {
                          li { key: "{album.id}",
                            AlbumResult { on_click: move |id| view_full_album(id), album: album.clone() }
                          }
                        },
                    }
                  }
                }
              },
              _ => rsx! {
                div { class: "text-center text-gray-500 py-10", "Search for something to see results here." }
              },
          }
        }
      }
    }
}
