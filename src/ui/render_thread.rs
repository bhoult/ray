//! Rendering off the UI thread: the UI posts the latest job (scene, camera,
//! settings, size); the thread renders it, accumulating while it does not
//! change and sleeping once converged, and hands back the newest frame. A slow
//! scene then slows only the image, not the panels.

use crate::camera::Camera;
use crate::render::{FrameStats, ObjCost, Renderer, View, Waves};
use crate::scene::Scene;
use crate::settings::Settings;
use eframe::egui;
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

#[derive(Clone)]
pub struct Job {
    pub scene: Arc<Scene>,
    pub scene_id: u64,
    pub cam: Camera,
    pub s: Settings,
    pub w: usize,
    pub h: usize,
    pub view: View,
    pub waves: Waves,
    /// counts the jobs posted (outputs carry it back)
    pub serial: u64,
    /// the share of the time it may keep the GPU and the CPU busy (1: all
    /// of it): after each pass the thread rests for the rest
    pub load: f32,
}

impl Job {
    /// Whether two jobs render the same image.
    pub fn same(&self, b: &Job) -> bool {
        self.scene_id == b.scene_id && self.cam == b.cam && self.s == b.s && self.w == b.w && self.h == b.h && self.view == b.view && self.waves == b.waves
    }
}

pub struct Output {
    /// shared with the texture and screenshots (no copies)
    pub image: Arc<egui::ColorImage>,
    pub stats: FrameStats,
    /// per scene object
    pub costs: Vec<ObjCost>,
    /// the cost view: per object, nanoseconds per evaluation (Renderer::prices)
    pub prices: Vec<f32>,
    pub scene_id: u64,
    /// the job's serial
    pub serial: u64,
    /// the GPU's state: its name, and an error when it cannot render
    pub gpu: Option<(String, Option<String>, f32, bool)>,
}

struct Shared {
    job: Mutex<Option<Job>>,
    wake: Condvar,
    out: Mutex<Option<Output>>,
    quit: AtomicBool,
    /// no more samples of an unchanged image for now (a new job still renders)
    paused: AtomicBool,
}

pub struct RenderThread {
    shared: Arc<Shared>,
    handle: Option<JoinHandle<()>>,
}

impl RenderThread {
    pub fn start(ctx: egui::Context) -> RenderThread {
        let shared = Arc::new(Shared {
            job: Mutex::new(None),
            wake: Condvar::new(),
            out: Mutex::new(None),
            quit: AtomicBool::new(false),
            paused: AtomicBool::new(false),
        });
        let sh = shared.clone();
        let handle = std::thread::Builder::new()
            .name("render".into())
            .spawn(move || run(sh, ctx))
            .expect("render thread");
        RenderThread { shared, handle: Some(handle) }
    }

    /// Replaces the pending job.
    pub fn post(&self, job: Job) {
        *self.shared.job.lock().unwrap() = Some(job);
        self.shared.wake.notify_one();
    }

    /// Stops (or lets go on) accumulating samples of an unchanged image: the
    /// window waiting for a screenshot of itself gets the GPU (a GPU kept busy
    /// by another program as well left the window a frame a second, and the
    /// screenshot was given up on).
    pub fn pause(&self, on: bool) {
        if self.shared.paused.swap(on, Ordering::Relaxed) != on && !on {
            self.shared.wake.notify_one();
        }
    }

    /// The newest finished frame, if there is one the UI has not taken.
    pub fn take(&self) -> Option<Output> {
        self.shared.out.lock().unwrap().take()
    }
}

impl Drop for RenderThread {
    fn drop(&mut self) {
        self.shared.quit.store(true, Ordering::Relaxed);
        self.shared.wake.notify_one();
        if let Some(h) = self.handle.take() {
            let _ = h.join();
        }
    }
}

/// A pass of a job kept to a share of the time (`Job::load`) takes about
/// this long, so that its rests come several times a second: a monitor then
/// sees the share, not seconds flat out and a pause.
const BURST_MS: f32 = 150.0;

fn run(sh: Arc<Shared>, ctx: egui::Context) {
    let mut renderer = Renderer::new();
    // a device of its own, not the window's: work on one queue runs in order,
    // so sharing would hold every traced frame behind the window's wait for
    // the display (and frames come back to the CPU for post anyway)
    renderer.gpu = match crate::gpu::Gpu::headless() {
        Ok(mut g) => {
            eprintln!("ray: render device ready {} ms after start", super::since_start());
            // (the CPU renders while the shader builds)
            g.background = true;
            Some(g)
        }
        Err(e) => {
            eprintln!("ray: {e}; rendering on the CPU only");
            None
        }
    };
    let mut cur: Option<Job> = None;
    let mut idle = true;
    // the last traced pass's milliseconds a sample a pixel
    let mut sample_ms = 0.0f32;
    loop {
        {
            let mut pending = sh.job.lock().unwrap();
            // sleep while there is nothing new and the current image is finished
            while pending.is_none() && (cur.is_none() || idle || sh.paused.load(Ordering::Relaxed)) && !sh.quit.load(Ordering::Relaxed) {
                pending = sh.wake.wait(pending).unwrap();
            }
            if sh.quit.load(Ordering::Relaxed) {
                return;
            }
            if let Some(j) = pending.take() {
                cur = Some(j);
            }
        }
        let job = cur.as_ref().unwrap();
        let load = job.load.clamp(0.1, 1.0);
        let mut s = job.s;
        if load < 1.0 && sample_ms > 0.0 {
            s.spp = ((BURST_MS / sample_ms) as u32).clamp(1, s.spp.max(1));
        }
        let started = Instant::now();
        let frame = renderer.render(&job.scene, &job.cam, &s, job.w, job.h, job.scene_id, &job.view, &job.waves);
        // to RGBA on every core (a single thread would leave the others idle between frames)
        let mut pixels = vec![egui::Color32::BLACK; frame.w * frame.h];
        pixels.par_chunks_mut(frame.w).enumerate().for_each(|(row, dst)| {
            for (d, &p) in dst.iter_mut().zip(&frame.out[row * frame.pw..][..frame.w]) {
                *d = egui::Color32::from_rgb((p >> 16) as u8, (p >> 8) as u8, p as u8);
            }
        });
        let out = Output {
            image: Arc::new(egui::ColorImage::new([frame.w, frame.h], pixels)),
            stats: renderer.stats,
            costs: renderer.costs.clone(),
            prices: renderer.prices.clone(),
            scene_id: job.scene_id,
            serial: job.serial,
            gpu: renderer.gpu.as_ref().map(|g| (g.name.clone(), g.error.clone(), g.compile_ms, g.compiling())),
        };
        idle = !renderer.stats.traced;
        *sh.out.lock().unwrap() = Some(out);
        ctx.request_repaint();
        if renderer.stats.traced {
            sample_ms = started.elapsed().as_secs_f32() * 1000.0 / s.spp.clamp(1, 64) as f32;
        }
        // a share of the time: the GPU and the CPU idle for the rest (a long
        // recording leaves the computer usable meanwhile)
        if load < 1.0 && renderer.stats.traced {
            let until = Instant::now() + started.elapsed().mul_f32((1.0 - load) / load);
            let mut g = sh.job.lock().unwrap();
            while !sh.quit.load(Ordering::Relaxed) && Instant::now() < until {
                g = sh.wake.wait_timeout(g, until - Instant::now()).unwrap().0;
            }
        }
    }
}
