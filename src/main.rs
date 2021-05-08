use std::error::Error;
use zbus::dbus_proxy;
use structopt::StructOpt;
use serde::{Serialize, Deserialize};
use std::time::SystemTime;
use std::path::Path;

#[derive(Serialize, Deserialize)]
struct Metadata {
    title: String,
    artist: String,
    length: i64,
    position: i64,
    timestamp: SystemTime
}

#[dbus_proxy(
    interface = "org.mpris.MediaPlayer2.Player",
    default_service = "org.mpris.MediaPlayer2.spotifyd",
    default_path = "/org/mpris/MediaPlayer2"
)]
trait Player {
    fn next(
        &self,
    ) -> zbus::Result<()>;
    fn previous(
        &self
    ) -> zbus::Result<()>;
    fn pause(
        &self
    ) -> zbus::Result<()>;
    fn play(
        &self
    ) -> zbus::Result<()>;
    fn play_pause(&self) -> zbus::Result<()>;
    #[dbus_proxy(property)]
    fn position(&self) -> zbus::Result<i64>;
    #[dbus_proxy(property)]
    fn metadata(&self) -> zbus::Result<zvariant::Dict>;
    #[dbus_proxy(property)]
    fn playback_status(&self) -> zbus::Result<String>;
}

fn get_proxy<'a>() -> Result<PlayerProxy<'a>, Box<dyn Error>> {
    let connection = zbus::Connection::new_session()?;
    let spotify_bus = PlayerProxy::new(&connection)?;

    Ok(spotify_bus)
}

fn get_cache() -> Result<Metadata, Box<dyn Error>> {
    let path = Path::new("/tmp/spotify_manage_cache");
    let cache_str = std::fs::read_to_string(path)?;
    let data: Metadata = serde_json::from_str(cache_str.as_str())?;

    Ok(data)
}

fn get_metadata(proxy: Option<PlayerProxy>) -> Result<Metadata, Box<dyn Error>> {
    let p_proxy = match proxy {
        Some (p) => p,
        None => get_proxy()?
    };

    // Check for metadata
    let possible_data: Option<Metadata> = match get_cache() {
        // Cache data exists
        Ok (cache_data) => if cache_data.timestamp.elapsed()?.as_secs() < 3 {
            Some(cache_data)
        } else {
            None
        },
        // Cache data does not exist, get metadata from API
        Err (_) => None
    };

    match possible_data {
        Some (data) => Ok(data),
        None => {
            let raw_metadata = p_proxy.metadata()?;

            let title = raw_metadata.get::<str, str>("xesam:title")?.ok_or("Invalid bus data")?;
            
            let artist = raw_metadata.get::<str, zvariant::Array>("xesam:artist")?
                .ok_or("Invalid dbus data")?
                .get()[0]
                .downcast_ref::<str>()
                .ok_or("Invalid dbus data")?;

            
            let length = *raw_metadata.get::<str, zvariant::Value>("mpris:length")?
                .ok_or("Invalid dbus data")?
                .downcast_ref::<i64>()
                .ok_or("Invalid dbus data")?;
           
            let position = p_proxy.position()?;

            let final_metadata = Metadata {
                title: String::from(title),
                artist: String::from(artist),
                length,
                position,
                timestamp: SystemTime::now()
            };

            let meta_as_string = serde_json::to_string(&final_metadata)?;
            let meta_as_bytes = meta_as_string.into_bytes();

            let path = Path::new("/tmp/spotify_manage_cache");

            std::fs::write(path, meta_as_bytes)?;

            Ok(final_metadata) 
        }
    }
}

fn get_song_progress() -> Result<f64, Box<dyn Error>> {
    let metadata = get_metadata(None)?;
    let current_pos = metadata.position as f64;
    let song_length = metadata.length as f64;


    Ok(current_pos / song_length)
}

fn get_song_name() -> Result<String, Box<dyn Error>> {
    let metadata = match get_metadata(None) {
        Ok(data) => data,
        Err(_) => get_cache()?
    }; 
    let artist = metadata.artist;
    let song_name = metadata.title;

    Ok(format!("{} - {}", artist, song_name))
}

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(long)]
    play: bool,
    #[structopt(long)]
    pause: bool,
    #[structopt(long)]
    next: bool,
    #[structopt(long)]
    progress: bool,
    #[structopt(long)]
    song: bool,
    #[structopt(long)]
    status: bool,
    #[structopt(long)]
    playpause: bool
}

fn main() -> Result<(), Box<dyn Error>>{
    let opt = Opt::from_args();
    let connection = zbus::Connection::new_session()?;
    let player = PlayerProxy::new(&connection)?;

    if opt.play {
        player.play()?
    }

    if opt.pause {
        player.pause()?
    }

    if opt.next {
        player.next()?
    }

    if opt.progress {
        println!("{}", get_song_progress()?)
    }

    if opt.song {
        println!("{}", get_song_name()?)
    }

    if opt.status {
        println!("{}", player.playback_status()?)
    }

    if opt.playpause {
        player.play_pause()?
    }
    
    Ok(())
}
