//! Recording a video of the window: every frame as drawn (the interface, the
//! view, game mode) with the pointer drawn in where it is, encoded by ffmpeg
//! (NVENC when it has it, else x264) into the videos folder, with the default
//! microphone (PulseAudio / PipeWire) alongside when asked. Frames and sound
//! are stamped by the wall clock as they arrive, so the narration stays with
//! the picture whatever the frame rate; the file runs at a steady frame rate
//! (a slower frame is shown for longer). Where, how many frames a second and
//! how sharp: the preferences (config.rs).
//!
//! A steady recording (the preference, or render_video over MCP) is made the
//! other way round: every frame of the video is the view drawn whole, the
//! scene 1/fps later than in the one before, however long the frames take to
//! draw (`Steady`). A scene that draws at 8 frames a second then plays
//! smoothly at 30, recorded slower than it plays. It is of the view, as
//! rendered (not the window: no panels, game mode's crosshair or pointer; a
//! busy GPU kept the window from drawing more than a frame a second), at a
//! size of its own (1920 x 1080, say, whatever the window's), each frame
//! with the samples a pixel asked for, and without sound (it would not keep
//! time with the picture).

use eframe::egui;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{SyncSender, sync_channel};
use std::thread::JoinHandle;
use std::time::Instant;

/// What a window screenshot for the recording is asked with (egui hands it
/// back with the image), so it is told from other screenshots.
#[derive(Clone, Copy, Debug)]
pub struct Tag;

/// A frame for the video: the window's pixels and where the pointer is on
/// them (pixels), when it shows.
struct Frame {
    image: Arc<egui::ColorImage>,
    pointer: Option<(f32, f32)>,
    scale: f32,
}

pub struct Recorder {
    pub path: PathBuf,
    pub started: Instant,
    pub mic: bool,
    pub steady: bool,
    tx: Option<SyncSender<Frame>>,
    writer: Option<JoinHandle<Result<String, String>>>,
}

impl Recorder {
    /// Starts ffmpeg for frames `size` pixels (the window's now: later
    /// frames of another size are cropped or padded to it), into folder
    /// `dir`, with the default microphone when `mic`, at `fps` frames a
    /// second and `quality` (the encoder's cq / crf: lower is sharper).
    /// `steady`: each frame is the next 1/fps of the video, however late it
    /// comes, and none is dropped (no microphone then; x264's slow preset,
    /// as the frames come slowly anyway).
    pub fn start(dir: &std::path::Path, size: [usize; 2], mic: bool, fps: u32, quality: u32, steady: bool) -> Result<Recorder, String> {
        let mic = mic && !steady;
        let fps = fps.clamp(5, 120).to_string();
        std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        // (whole: the path is reported, and looked for once the file is done)
        let dir = &std::path::absolute(dir).unwrap_or_else(|_| dir.to_path_buf());
        // (a second one within the same second: -2, -3...)
        let base = format!("ray-{}", stamp());
        let mut stem = dir.join(&base);
        for k in 2.. {
            if !stem.with_extension("mp4").exists() {
                break;
            }
            stem = dir.join(format!("{base}-{k}"));
        }
        let path = stem.with_extension("mp4");
        let log = stem.with_extension("log");
        // (yuv420p wants even sides)
        let (w, h) = (size[0] & !1, size[1] & !1);
        if w < 16 || h < 16 {
            return Err("the window is too small to record".into());
        }
        let mut cmd = Command::new("ffmpeg");
        cmd.args(["-hide_banner", "-loglevel", "warning", "-y"]);
        if steady {
            cmd.args(["-thread_queue_size", "64", "-framerate", &fps]);
        } else {
            cmd.args(["-thread_queue_size", "64", "-use_wallclock_as_timestamps", "1"]);
        }
        cmd.args(["-f", "rawvideo", "-pix_fmt", "rgba", "-video_size", &format!("{w}x{h}"), "-i", "pipe:0"]);
        if mic {
            cmd.args(["-thread_queue_size", "4096", "-f", "pulse", "-i", "default"]);
        }
        // both on the wall clock, from the first of them
        if !steady {
            cmd.args(["-copyts", "-start_at_zero", "-fps_mode", "cfr", "-r", &fps]);
        }
        let q = quality.clamp(0, 51).to_string();
        // (NVENC refuses frames below its minimum size, about 145 x 49: a
        // small window is encoded by x264)
        if steady {
            cmd.args(["-c:v", "libx264", "-preset", "slow", "-crf", &q]);
        } else if nvenc() && w >= 256 && h >= 128 {
            cmd.args(["-c:v", "h264_nvenc", "-preset", "p5", "-tune", "hq", "-rc", "vbr", "-cq", &q, "-b:v", "0"]);
        } else {
            cmd.args(["-c:v", "libx264", "-preset", "veryfast", "-crf", &q]);
        }
        cmd.args(["-pix_fmt", "yuv420p"]);
        if mic {
            // (the sound would go on for ever: it ends with the picture)
            cmd.args(["-c:a", "aac", "-b:a", "160k", "-shortest"]);
        }
        cmd.args(["-movflags", "+faststart"]).arg(&path);
        let err = std::fs::File::create(&log).map_err(|e| format!("{}: {e}", log.display()))?;
        cmd.stdin(Stdio::piped()).stdout(Stdio::null()).stderr(err);
        let mut child = cmd.spawn().map_err(|e| format!("ffmpeg: {e} (is it installed?)"))?;
        let mut stdin = child.stdin.take().ok_or("ffmpeg: no input pipe")?;
        // a few frames may wait; more are dropped (the clock keeps the time)
        let (tx, rx) = sync_channel::<Frame>(3);
        let out = path.clone();
        let writer = std::thread::Builder::new()
            .name("record".into())
            .spawn(move || {
                let mut buf = vec![0u8; w * h * 4];
                let mut wrote = false;
                for f in rx {
                    compose(&mut buf, w, h, &f);
                    if stdin.write_all(&buf).is_err() {
                        break;
                    }
                    wrote = true;
                }
                drop(stdin);
                let status = child.wait().map_err(|e| e.to_string())?;
                let said = std::fs::read_to_string(&log).unwrap_or_default();
                if status.success() && wrote {
                    let _ = std::fs::remove_file(&log);
                    Ok(out.display().to_string())
                } else {
                    let last = said.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or("no frames").to_string();
                    Err(format!("{last} (see {})", log.display()))
                }
            })
            .map_err(|e| e.to_string())?;
        Ok(Recorder { path, started: Instant::now(), mic, steady, tx: Some(tx), writer: Some(writer) })
    }

    /// Hands a frame of the window over, with the pointer at `pointer`
    /// (points; None: hidden) and `scale` pixels per point. One that finds
    /// the encoder behind is dropped, unless the recording is steady (it
    /// waits).
    pub fn frame(&self, image: Arc<egui::ColorImage>, pointer: Option<egui::Pos2>, scale: f32) {
        if let Some(tx) = &self.tx {
            let f = Frame { image, pointer: pointer.map(|p| (p.x * scale, p.y * scale)), scale };
            if self.steady {
                let _ = tx.send(f);
            } else {
                let _ = tx.try_send(f);
            }
        }
    }

    /// Whether the encoder has stopped by itself (ffmpeg failed: no
    /// microphone server, an encoder it cannot open, a full disk...); `stop`
    /// then says why.
    pub fn failed(&self) -> bool {
        self.writer.as_ref().is_some_and(|w| w.is_finished())
    }

    /// Ends the recording; the file is finished in the background, and the
    /// handle says how it went (its path, or what went wrong).
    pub fn stop(mut self) -> Option<JoinHandle<Result<String, String>>> {
        drop(self.tx.take());
        self.writer.take()
    }
}

/// A steady recording's clock: the scene moves 1/fps a frame of the video,
/// and only once the frame before is in it (the view drawn whole, with the
/// samples asked for).
pub struct Steady {
    pub fps: u32,
    /// samples a pixel each frame waits for (0: as many as the render
    /// settings' max_samples, the frame converged)
    pub samples: u32,
    /// the frames' size, pixels (the view renders at it, whatever the
    /// window's)
    pub size: [usize; 2],
    /// the frames it ends after (None: when it is stopped)
    pub until: Option<u64>,
    /// frames in the video so far
    pub frames: u64,
    /// the scene's clock, seconds: `start` and frames/fps on (worked out,
    /// not added up: a viewer up for hours keeps its clock in the
    /// thousands of seconds, where adding 1/fps a frame in f32 drifted a
    /// frame in a few hundred)
    pub t: f64,
    start: f64,
    /// when it started: how long its frames take to draw
    pub began: Instant,
    /// the share of the time the renderer may keep the GPU and the CPU busy
    /// meanwhile (1: all of it)
    pub load: f32,
    /// the frame whose view has gone to the renderer (it is not sent again
    /// until that frame is in the video)
    pub posted_for: Option<u64>,
}

impl Steady {
    pub fn new(fps: u32, samples: u32, size: [usize; 2], until: Option<u64>, t: f64) -> Steady {
        Steady { fps: fps.clamp(5, 120), samples, size, until, frames: 0, t, start: t, began: Instant::now(), load: 1.0, posted_for: None }
    }

    /// The samples a pixel each frame waits for, with the render settings'
    /// `max_samples`.
    pub fn wants(&self, max_samples: u32) -> u32 {
        if self.samples == 0 { max_samples.max(1) } else { self.samples }
    }

    /// Whether it has all its frames.
    pub fn done(&self) -> bool {
        self.until.is_some_and(|n| self.frames >= n)
    }

    /// Seconds a frame has taken to draw, on average (the wall clock's).
    pub fn secs_a_frame(&self) -> f32 {
        self.began.elapsed().as_secs_f32() / self.frames.max(1) as f32
    }

    /// The view is drawn whole (`drawn`): that frame goes into the video,
    /// and the scene moves on 1/fps. Whether it does.
    pub fn tick(&mut self, drawn: bool) -> bool {
        if drawn {
            self.frames += 1;
            self.t = self.start + self.frames as f64 / self.fps as f64;
        }
        drawn
    }
}

/// `f` into `buf` (`w` x `h` RGBA: cropped or padded with black), and the
/// pointer drawn on it.
fn compose(buf: &mut [u8], w: usize, h: usize, f: &Frame) {
    let [iw, ih] = f.image.size;
    let px: &[u8] = bytes(&f.image.pixels);
    let row = w.min(iw) * 4;
    for y in 0..h {
        let dst = &mut buf[y * w * 4..(y + 1) * w * 4];
        if y < ih {
            dst[..row].copy_from_slice(&px[y * iw * 4..y * iw * 4 + row]);
            dst[row..].fill(0);
        } else {
            dst.fill(0);
        }
    }
    if let Some((x, y)) = f.pointer {
        pointer(buf, w, h, x, y, f.scale.round().max(1.0) as usize);
    }
}

/// egui's pixels as bytes (Color32 is four bytes, RGBA).
fn bytes(px: &[egui::Color32]) -> &[u8] {
    // SAFETY: Color32 is a [u8; 4] with no padding
    unsafe { std::slice::from_raw_parts(px.as_ptr() as *const u8, std::mem::size_of_val(px)) }
}

/// The desktop's arrow, drawn at (`x`, `y`) `k` pixels to its pixel: the
/// desktop draws its pointer above the window, so a capture of the window
/// has none.
fn pointer(buf: &mut [u8], w: usize, h: usize, x: f32, y: f32, k: usize) {
    const ARROW: [&str; 17] = [
        "X",
        "XX",
        "X.X",
        "X..X",
        "X...X",
        "X....X",
        "X.....X",
        "X......X",
        "X.......X",
        "X........X",
        "X.....XXXXX",
        "X..X..X",
        "X.X X..X",
        "XX  X..X",
        "X    X..X",
        "     X..X",
        "      XX",
    ];
    let (x0, y0) = (x.round() as isize, y.round() as isize);
    for (r, line) in ARROW.iter().enumerate() {
        for (c, ch) in line.chars().enumerate() {
            let rgb = match ch {
                'X' => [0u8, 0, 0],
                '.' => [255u8, 255, 255],
                _ => continue,
            };
            for dy in 0..k {
                for dx in 0..k {
                    let (px, py) = (x0 + (c * k + dx) as isize, y0 + (r * k + dy) as isize);
                    if px >= 0 && py >= 0 && (px as usize) < w && (py as usize) < h {
                        let i = (py as usize * w + px as usize) * 4;
                        buf[i..i + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
                    }
                }
            }
        }
    }
}

/// Whether NVIDIA's encoder works here: tried once on a few blank frames
/// (ffmpeg lists it whether or not there is an NVIDIA driver to run it).
/// `probe` at start does it in the background, so recording does not wait.
pub fn nvenc() -> bool {
    static HAS: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *HAS.get_or_init(|| {
        Command::new("ffmpeg")
            .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "color=black:s=256x256:d=0.2", "-c:v", "h264_nvenc", "-f", "null", "-"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    })
}

/// Finds out in the background whether NVIDIA's encoder works (nvenc).
pub fn probe() {
    std::thread::spawn(nvenc);
}

/// Now, local time, for a file name: 2026-10-29-0931-07.
pub fn stamp() -> String {
    Command::new("date")
        .arg("+%Y-%m-%d-%H%M-%S")
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()).to_string())
}

/// m:ss
pub fn clock(secs: f32) -> String {
    let s = secs.max(0.0) as u64;
    format!("{}:{:02}", s / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn image(w: usize, h: usize, c: egui::Color32) -> Arc<egui::ColorImage> {
        Arc::new(egui::ColorImage::new([w, h], vec![c; w * h]))
    }

    fn at(buf: &[u8], w: usize, x: usize, y: usize) -> [u8; 4] {
        let i = (y * w + x) * 4;
        [buf[i], buf[i + 1], buf[i + 2], buf[i + 3]]
    }

    /// A frame is put into the video's size: a smaller one padded with
    /// black, a bigger one cropped; the pointer's arrow drawn where it is
    /// (black outline, white inside), at the screen's scale.
    #[test]
    fn frames_fit_the_video_and_show_the_pointer() {
        let red = egui::Color32::from_rgb(200, 0, 0);
        let (w, h) = (24, 20);
        let mut buf = vec![7u8; w * h * 4];
        compose(&mut buf, w, h, &Frame { image: image(10, 8, red), pointer: None, scale: 1.0 });
        assert_eq!(at(&buf, w, 9, 7), [200, 0, 0, 255]);
        assert_eq!(at(&buf, w, 10, 7), [0, 0, 0, 0], "padded right");
        assert_eq!(at(&buf, w, 3, 8), [0, 0, 0, 0], "padded below");
        compose(&mut buf, w, h, &Frame { image: image(40, 30, red), pointer: Some((2.0, 3.0)), scale: 1.0 });
        assert_eq!(at(&buf, w, w - 1, h - 1), [200, 0, 0, 255], "cropped, no padding");
        assert_eq!(at(&buf, w, 2, 3), [0, 0, 0, 255], "the arrow's tip");
        assert_eq!(at(&buf, w, 3, 5), [255, 255, 255, 255], "inside the arrow");
        // at twice the scale each of its pixels is two
        compose(&mut buf, w, h, &Frame { image: image(40, 30, red), pointer: Some((2.0, 3.0)), scale: 2.0 });
        assert_eq!(at(&buf, w, 3, 4), [0, 0, 0, 255]);
        assert_eq!(at(&buf, w, 4, 7), [255, 255, 255, 255]);
        // off the frame's edge, nothing breaks
        compose(&mut buf, w, h, &Frame { image: image(40, 30, red), pointer: Some((w as f32 - 1.0, h as f32 - 1.0)), scale: 3.0 });
    }

    #[test]
    fn clocks_and_names() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(61.9), "1:01");
        assert_eq!(clock(-3.0), "0:00");
        let s = stamp();
        let shape = s.len() == 18 && s.chars().enumerate().all(|(i, c)| if [4, 7, 10, 15].contains(&i) { c == '-' } else { c.is_ascii_digit() });
        assert!(shape, "a file-name stamp like 2026-10-29-0931-07: {s}");
    }

    /// A short recording through ffmpeg: frames in, an .mp4 out, its log
    /// gone (it stays only when something went wrong); a window too small
    /// for NVIDIA's encoder too (x264 takes it). Without ffmpeg it says so
    /// and passes.
    #[test]
    fn records_a_video() {
        if Command::new("ffmpeg").arg("-version").stdout(Stdio::null()).stderr(Stdio::null()).status().is_err() {
            eprintln!("no ffmpeg: recording is not tested");
            return;
        }
        let dir = std::env::temp_dir().join(format!("ray-test-{}-record", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut paths = Vec::new();
        for (w, h) in [(64, 48), (320, 240)] {
            let r = Recorder::start(&dir, [w, h], false, 30, 30, false).unwrap();
            for k in 0..20u8 {
                r.frame(image(w, h, egui::Color32::from_rgb(k * 12, 80, 160)), Some(egui::pos2(10.0, 10.0)), 1.0);
                std::thread::sleep(std::time::Duration::from_millis(20));
            }
            assert!(!r.failed(), "{w} x {h}: the encoder stopped by itself");
            let path = r.path.clone();
            let done = r.stop().unwrap().join().unwrap();
            assert_eq!(done.as_deref(), Ok(path.display().to_string().as_str()), "{w} x {h}");
            let size = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
            assert!(size > 500, "{w} x {h}: a video of {size} bytes");
            assert!(!path.with_extension("log").exists(), "{w} x {h}: the log is removed when it went well");
            paths.push(path);
        }
        // (both within a second: the second named apart, not over the first)
        assert_ne!(paths[0], paths[1]);
        assert!(paths.iter().all(|p| p.exists()));
        // steady: every frame handed over is one of the video, at its rate,
        // however slowly they come
        let r = Recorder::start(&dir, [320, 240], true, 30, 30, true).unwrap();
        assert!(!r.mic, "a steady recording takes no sound");
        for k in 0..12u8 {
            r.frame(image(320, 240, egui::Color32::from_rgb(k * 20, 80, 160)), None, 1.0);
            std::thread::sleep(std::time::Duration::from_millis(if k % 4 == 0 { 120 } else { 5 }));
        }
        let path = r.path.clone();
        assert!(r.stop().unwrap().join().unwrap().is_ok());
        let probe = Command::new("ffprobe")
            .args(["-v", "error", "-count_frames", "-select_streams", "v:0", "-show_entries", "stream=nb_read_frames,r_frame_rate", "-of", "csv=p=0"])
            .arg(&path)
            .output();
        if let Ok(o) = probe {
            let said = String::from_utf8_lossy(&o.stdout).trim().to_string();
            assert_eq!(said, "30/1,12", "the steady video's rate and frames");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// A steady recording moves on a frame (1/fps of the scene's clock) each
    /// time the view is drawn whole, and waits otherwise.
    #[test]
    fn steady_steps_once_each_frame_is_drawn() {
        let mut s = Steady::new(30, 4, [64, 48], Some(3), 10.0);
        assert!(!s.tick(false), "not drawn yet");
        assert!(s.tick(true));
        assert!(!s.tick(false));
        assert!(s.tick(true));
        assert_eq!(s.frames, 2);
        assert!((s.t - (10.0 + 2.0 / 30.0)).abs() < 1e-5, "{}", s.t);
        assert!(!s.done());
        assert!(s.tick(true) && s.done(), "done after its frames");
        assert_eq!(Steady::new(1000, 4, [64, 48], None, 0.0).fps, 120, "(clamped)");
        // 0 samples: until the frame converges
        assert_eq!((s.wants(1024), Steady::new(30, 0, [64, 48], None, 0.0).wants(1024)), (4, 1024));
        // a viewer up for three hours: after 7 s of frames its clock is 7 s
        // on, to the microsecond (added up in f32 it was a frame off)
        let mut s = Steady::new(30, 4, [64, 48], None, 10_800.0);
        for _ in 0..210 {
            s.tick(true);
        }
        assert!((s.t - 10_807.0).abs() < 1e-6, "{}", s.t);
    }
}
