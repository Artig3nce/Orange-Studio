//! Freesound in the browser (https://freesound.org/docs/api/). The Freesound section searches with the browser's search
//! text (a click on its "Search Freesound" row). The search and each preview download run on their own threads; the
//! app collects their results in `poll` on its tick. A clicked sound's preview is downloaded once into a cache folder,
//! decoded for its waveform, and played through the browser's preview.
//!
//! The API key is read from `~/.orange-studio/freesound_token.txt` (one line), never stored in the project.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

use os_engine::Sample;

/// Tree key of the Freesound root in the browser's All tab.
pub const PREFIX: &str = "0:Freesound";

/// One search result: the fields the browser shows and plays.
#[derive(Debug, Clone, PartialEq)]
pub struct Sound {
    pub id: u64,
    pub name: String,
    pub duration: f32,
    /// The low-quality MP3 preview's URL.
    pub preview: String,
}

/// The Freesound section's state (`State::freesound`).
#[derive(Debug, Clone, Default)]
pub struct Fs {
    /// What the section says under its search row.
    pub status: String,
    pub results: Vec<Sound>,
    /// Waveform peaks of each downloaded preview, 0..1 per bin.
    pub peaks: HashMap<u64, Vec<f32>>,
    /// Cached preview files by sound id.
    pub files: HashMap<u64, PathBuf>,
    /// A clicked sound whose preview is still downloading: it plays when the download is done.
    pub waiting: Option<u64>,
    /// Sounds whose preview is downloading (all of a new search's results, one after the other).
    pub loading: HashSet<u64>,
    pub busy: bool,
    /// The sound being heard now (its card shows a stop mark and a moving line over the waveform).
    pub playing: Option<Playing>,
    /// A card's playhead being dragged: the sound and its share of the waveform (0..1). The preview moves there when
    /// the button is let go.
    pub scrub: Option<(u64, f32)>,
    /// Where a waveform was clicked while its preview was still downloading (share of the sound, 0..1): `poll` starts
    /// the preview there once the file is in.
    pub waiting_from: Option<f32>,
}

/// A preview that sounds: which sound, since when, and for how long (the preview is cut off after a few seconds unless
/// "Full sample preview" is on), and how far into the sound this play began.
#[derive(Debug, Clone, Copy)]
pub struct Playing {
    pub id: u64,
    pub since: Instant,
    pub secs: f32,
    pub duration: f32,
    /// Seconds into the sound where this play started (0 for a normal preview, the playhead's place after a seek).
    pub from: f32,
}

/// Sound `id` starts sounding now, cut off after `limit` seconds when there is a limit.
pub fn start(s: &mut Fs, id: u64, limit: Option<f32>) {
    let duration = s.results.iter().find(|x| x.id == id).map_or(0.0, |x| x.duration);
    let secs = limit.map_or(duration, |l| duration.min(l));
    s.playing = Some(Playing { id, since: Instant::now(), secs, duration, from: 0.0 });
}

/// Sound `id` starts sounding at `from` seconds into it (the card's playhead was moved there).
pub fn start_at(s: &mut Fs, id: u64, from: f32) {
    let duration = s.results.iter().find(|x| x.id == id).map_or(0.0, |x| x.duration);
    let from = from.clamp(0.0, (duration - 0.05).max(0.0));
    s.playing = Some(Playing { id, since: Instant::now(), secs: duration, duration, from });
}

/// Where a click at `frac` (0..1 across the waveform) starts the preview in seconds.
pub fn offset(s: &Fs, id: u64, frac: f32) -> f32 {
    let duration = s.results.iter().find(|x| x.id == id).map_or(0.0, |x| x.duration);
    let from = frac.clamp(0.0, 1.0) * duration;
    from.clamp(0.0, (duration - 0.05).max(0.0))
}

/// How far into sound `id` the preview is (0..1 of the whole sound), while it sounds.
pub fn progress(s: &Fs, id: u64) -> Option<f32> {
    let p = s.playing.filter(|p| p.id == id)?;
    let t = p.from + p.since.elapsed().as_secs_f32();
    (t < p.secs).then(|| (t / p.duration.max(0.01)).min(1.0))
}

/// Called on every tick: ends a finished preview. Returns whether the cards need repainting (a preview sounds).
pub fn tick(s: &mut Fs) -> bool {
    match s.playing {
        Some(p) if p.from + p.since.elapsed().as_secs_f32() >= p.secs => {
            s.playing = None;
            true
        }
        Some(_) => true,
        None => false,
    }
}

enum Msg {
    Results(Result<Vec<Sound>, String>),
    Ready(u64, PathBuf, Vec<f32>),
    Failed(u64, String),
}

/// Results of the background threads, waiting for `poll`.
fn inbox() -> &'static Mutex<Vec<Msg>> {
    static INBOX: OnceLock<Mutex<Vec<Msg>>> = OnceLock::new();
    INBOX.get_or_init(|| Mutex::new(Vec::new()))
}

fn post(m: Msg) {
    if let Ok(mut q) = inbox().lock() {
        q.push(m);
    }
}

/// The API key, from the user's own file.
pub fn token() -> Result<String, String> {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME")).map(PathBuf::from).ok_or_else(|| "Freesound: no home folder".to_string())?;
    let file = home.join(".orange-studio").join("freesound_token.txt");
    let text = std::fs::read_to_string(&file).map_err(|_| format!("Freesound: put your API key in {}", file.display()))?;
    let t = text.trim().to_string();
    if t.is_empty() { Err("Freesound: the API key file is empty".into()) } else { Ok(t) }
}

/// Starts a search for `query` in the background.
pub fn search(s: &mut Fs, query: &str) {
    let q = query.trim().to_string();
    if q.is_empty() {
        s.status = "Type a sound name in the search box first".into();
        return;
    }
    let tok = match token() {
        Ok(t) => t,
        Err(e) => {
            s.status = e;
            return;
        }
    };
    s.busy = true;
    s.scrub = None;
    s.waiting_from = None;
    s.status = format!("Searching Freesound for \"{q}\"...");
    std::thread::spawn(move || post(Msg::Results(fetch_search(&q, &tok))));
}

/// `curl` without a console window: the app has none of its own, and Windows opens (and closes) one for every child
/// process started without this flag, which flickered for each search and preview download.
fn curl() -> Command {
    let mut cmd = Command::new("curl");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    cmd
}

fn fetch_search(query: &str, token: &str) -> Result<Vec<Sound>, String> {
    let out = curl()
        .args(["-fsSL", "-G", "--max-time", "20"])
        .arg("--data-urlencode")
        .arg(format!("query={query}"))
        .arg("--data-urlencode")
        .arg("fields=id,name,duration,previews")
        .arg("--data-urlencode")
        .arg("page_size=30")
        .arg("--data-urlencode")
        .arg(format!("token={token}"))
        .arg("https://freesound.org/apiv2/search/")
        .output()
        .map_err(|e| format!("curl did not start: {e}"))?;
    if !out.status.success() {
        return Err("Freesound did not answer: check the API key and the connection".into());
    }
    parse(&out.stdout)
}

/// The results of a search response (`results`: id, name, duration, previews).
pub fn parse(json: &[u8]) -> Result<Vec<Sound>, String> {
    let v: serde_json::Value = serde_json::from_slice(json).map_err(|e| format!("Freesound: {e}"))?;
    let list = v.get("results").and_then(|r| r.as_array()).ok_or_else(|| "Freesound: no results in the answer".to_string())?;
    Ok(list
        .iter()
        .filter_map(|r| {
            Some(Sound {
                id: r.get("id")?.as_u64()?,
                name: r.get("name")?.as_str()?.to_string(),
                duration: r.get("duration").and_then(|d| d.as_f64()).unwrap_or(0.0) as f32,
                preview: r.get("previews")?.get("preview-lq-mp3")?.as_str()?.to_string(),
            })
        })
        .collect())
}

/// A sound row was clicked. Returns its preview file when it is already cached; otherwise starts the download and
/// returns `None` (the file plays when `poll` sees it).
pub fn click(s: &mut Fs, id: u64) -> Option<PathBuf> {
    s.waiting_from = None;
    s.scrub = None;
    if let Some(p) = s.files.get(&id) {
        let name = s.results.iter().find(|x| x.id == id).map_or("", |x| x.name.as_str());
        s.status = format!("Playing: {name}");
        return Some(p.clone());
    }
    let url = s.results.iter().find(|x| x.id == id)?.preview.clone();
    s.waiting = Some(id);
    // A preview already on its way plays when it arrives; otherwise fetch it now.
    if s.loading.insert(id) {
        std::thread::spawn(move || post(fetch_preview(id, &url)));
    }
    None
}

fn fetch_preview(id: u64, url: &str) -> Msg {
    let dir = std::env::temp_dir().join("Orange Studio").join("Freesound");
    if let Err(e) = std::fs::create_dir_all(&dir) {
        return Msg::Failed(id, format!("Freesound cache: {e}"));
    }
    let path = dir.join(format!("{id}.mp3"));
    if !path.exists() {
        // Download to a partial name first, so a cut download is never taken for a finished one.
        let part = dir.join(format!("{id}.part"));
        let ok = curl().args(["-fsSL", "--max-time", "30", "-o"]).arg(&part).arg(url).status();
        if !matches!(ok, Ok(st) if st.success()) {
            let _ = std::fs::remove_file(&part);
            return Msg::Failed(id, "The preview did not download".into());
        }
        if std::fs::rename(&part, &path).is_err() {
            return Msg::Failed(id, "The preview could not be saved".into());
        }
    }
    match os_engine::sample::load(&path) {
        Ok(sample) => Msg::Ready(id, path, peaks(&sample, WAVE_BINS)),
        Err(e) => Msg::Failed(id, format!("The preview did not decode: {e}")),
    }
}

/// Waveform bins kept per sound.
pub const WAVE_BINS: usize = 96;

/// The loudest sample of each of `bins` slices of the sound, scaled so the loudest bin is 1.
pub fn peaks(s: &Sample, bins: usize) -> Vec<f32> {
    let n = s.left.len().max(s.right.len());
    if n == 0 || bins == 0 {
        return vec![0.0; bins];
    }
    let mut out: Vec<f32> = (0..bins)
        .map(|i| {
            let a = i * n / bins;
            let b = ((i + 1) * n / bins).max(a + 1).min(n);
            (a..b).fold(0f32, |m, k| m.max(s.left.get(k).copied().unwrap_or(0.0).abs()).max(s.right.get(k).copied().unwrap_or(0.0).abs()))
        })
        .collect();
    let top = out.iter().copied().fold(0f32, f32::max).max(1e-6);
    for v in &mut out {
        *v /= top;
    }
    out
}

/// Takes the finished background work into `s`. Returns whether anything changed and the file to play, when a click
/// was waiting for its download: the sound, its file and the seconds into it to start at.
pub fn poll(s: &mut Fs) -> (bool, Option<(u64, PathBuf, f32)>) {
    let msgs = match inbox().lock() {
        Ok(mut q) => std::mem::take(&mut *q),
        Err(_) => return (false, None),
    };
    let changed = !msgs.is_empty();
    let mut play = None;
    for m in msgs {
        match m {
            Msg::Results(Ok(list)) => {
                s.busy = false;
                s.status = if list.is_empty() { "No sounds found".into() } else { if list.len() == 1 { "1 sound".to_string() } else { format!("{} sounds", list.len()) } };
                // Every preview loads one after the other in the background, so the waveforms fill in on their own.
                let todo: Vec<(u64, String)> = list.iter().filter(|x| !s.files.contains_key(&x.id) && s.loading.insert(x.id)).map(|x| (x.id, x.preview.clone())).collect();
                if !todo.is_empty() {
                    std::thread::spawn(move || {
                        for (id, url) in todo {
                            post(fetch_preview(id, &url));
                        }
                    });
                }
                s.results = list;
            }
            Msg::Results(Err(e)) => {
                s.busy = false;
                s.status = e;
            }
            Msg::Ready(id, path, bins) => {
                s.loading.remove(&id);
                s.files.insert(id, path.clone());
                s.peaks.insert(id, bins);
                if s.waiting == Some(id) {
                    s.waiting = None;
                    let name = s.results.iter().find(|x| x.id == id).map_or("", |x| x.name.as_str());
                    s.status = format!("Playing: {name}");
                    let from = s.waiting_from.take().map_or(0.0, |f| offset(s, id, f));
                    play = Some((id, path, from));
                }
            }
            Msg::Failed(id, e) => {
                s.loading.remove(&id);
                // Only a sound the user asked for reports its failure; the background ones just stay without a waveform.
                if s.waiting == Some(id) {
                    s.waiting = None;
                    s.status = e;
                }
            }
        }
    }
    (changed, play)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_search_answer() {
        let json = br#"{"count":2,"results":[{"id":5,"name":"Kick 1","duration":0.5,"previews":{"preview-lq-mp3":"https://x/5.mp3","preview-hq-mp3":"https://x/5h.mp3"}},{"id":6,"name":"no preview"}]}"#;
        let got = parse(json).unwrap();
        assert_eq!(got, vec![Sound { id: 5, name: "Kick 1".into(), duration: 0.5, preview: "https://x/5.mp3".into() }]);
    }

    #[test]
    fn the_playing_position_follows_the_clock_and_ends() {
        let mut s = Fs::default();
        s.results.push(Sound { id: 1, name: "a".into(), duration: 10.0, preview: String::new() });
        start(&mut s, 1, Some(5.0));
        assert!(progress(&s, 1).is_some_and(|p| p < 0.05));
        assert!(progress(&s, 2).is_none(), "another sound is not playing");
        assert!(tick(&mut s), "a sounding preview keeps the cards repainting");
        // Past the cut-off the preview is over.
        let back = std::time::Duration::from_secs(6);
        s.playing.as_mut().unwrap().since = Instant::now().checked_sub(back).unwrap_or_else(Instant::now);
        if s.playing.unwrap().since.elapsed() >= back {
            assert!(progress(&s, 1).is_none());
            assert!(tick(&mut s));
            assert!(s.playing.is_none());
            assert!(!tick(&mut s));
        }
    }

    #[test]
    fn a_seek_starts_the_position_at_the_playhead() {
        let mut s = Fs::default();
        s.results.push(Sound { id: 1, name: "a".into(), duration: 10.0, preview: String::new() });
        let from = offset(&s, 1, 0.4);
        assert!((from - 4.0).abs() < 1e-6);
        start_at(&mut s, 1, from);
        let p = s.playing.unwrap();
        assert!((p.from - 4.0).abs() < 1e-6 && p.secs == 10.0);
        assert!(progress(&s, 1).is_some_and(|x| (0.4..0.41).contains(&x)), "the line starts at the playhead");
    }

    #[test]
    fn peaks_scale_to_the_loudest_bin() {
        let s = Sample { rate: 44100, left: vec![0.1, 0.5, 0.0, 0.25], right: vec![0.0; 4] };
        let p = peaks(&s, 2);
        assert_eq!(p.len(), 2);
        assert!((p[0] - 1.0).abs() < 1e-6);
        assert!((p[1] - 0.5).abs() < 1e-6);
    }
}
