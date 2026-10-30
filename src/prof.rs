//! Profiling, built with `--features profile`: a sampling profiler (pprof,
//! driven by SIGPROF, so it needs no perf privileges) and an allocation
//! counter in place of the global allocator.
//!
//!   RAY_PROFILE=out.svg     write a flame graph of the run, and print the
//!                           functions and lines that took the most time
//!   RAY_PROFILE_ALLOCS=N    record where every Nth allocation was made, and
//!                           print the places that allocate the most
//!   RAY_PROFILE_THREAD=T    only the threads whose names start with T (the
//!                           viewer's: ray, render, trace N)
//!   RAY_PROFILE_TOP=N       how many of each to print (default 25)

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering::Relaxed};

struct Counting;

static ALLOCS: AtomicU64 = AtomicU64::new(0);
static BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);
/// the sampler's own tables, left out of the heap figures
static OWN: AtomicUsize = AtomicUsize::new(0);
/// record every Nth allocation's call stack (0: none)
static EVERY: AtomicU64 = AtomicU64::new(0);
static SITES: Mutex<Vec<(std::backtrace::Backtrace, usize)>> = Mutex::new(Vec::new());

thread_local! {
    /// set while this thread records a stack, whose own allocations are not
    /// recorded
    static RECORDING: Cell<bool> = const { Cell::new(false) };
}

fn grew(by: usize) {
    let n = ALLOCS.fetch_add(1, Relaxed);
    BYTES.fetch_add(by as u64, Relaxed);
    let live = LIVE.fetch_add(by, Relaxed) + by;
    PEAK.fetch_max(live, Relaxed);
    let every = EVERY.load(Relaxed);
    if every > 0 && n.is_multiple_of(every) && RECORDING.try_with(|r| !r.replace(true)).unwrap_or(false) {
        let bt = std::backtrace::Backtrace::force_capture();
        SITES.lock().unwrap().push((bt, by));
        RECORDING.with(|r| r.set(false));
    }
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(l) };
        if !p.is_null() {
            grew(l.size());
        }
        p
    }
    unsafe fn alloc_zeroed(&self, l: Layout) -> *mut u8 {
        let p = unsafe { System.alloc_zeroed(l) };
        if !p.is_null() {
            grew(l.size());
        }
        p
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        unsafe { System.dealloc(p, l) };
        LIVE.fetch_sub(l.size(), Relaxed);
    }
    unsafe fn realloc(&self, p: *mut u8, l: Layout, size: usize) -> *mut u8 {
        let q = unsafe { System.realloc(p, l, size) };
        if !q.is_null() {
            LIVE.fetch_sub(l.size(), Relaxed);
            grew(size);
        }
        q
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// Heap use so far: allocations made, bytes they asked for, bytes live now
/// and at most (without the profiler's own).
#[derive(Clone, Copy, Default)]
pub struct Heap {
    pub allocs: u64,
    pub bytes: u64,
    pub live: usize,
    pub peak: usize,
}

pub fn heap() -> Heap {
    let own = OWN.load(Relaxed);
    Heap {
        allocs: ALLOCS.load(Relaxed),
        bytes: BYTES.load(Relaxed),
        live: LIVE.load(Relaxed).saturating_sub(own),
        peak: PEAK.load(Relaxed).saturating_sub(own),
    }
}

/// The allocations since `since`, and the heap now, for a log line.
pub fn heap_since(since: &mut Heap) -> String {
    let h = heap();
    let s = format!(
        "{} allocs ({:.1} KB), live {:.1} MB, peak {:.1} MB",
        h.allocs - since.allocs,
        (h.bytes - since.bytes) as f64 / 1e3,
        h.live as f64 / 1e6,
        h.peak as f64 / 1e6
    );
    *since = h;
    s
}

pub struct Profiler {
    guard: Option<(pprof::ProfilerGuard<'static>, String)>,
    top: usize,
}

/// Starts sampling time when RAY_PROFILE names an output file, and
/// allocations when RAY_PROFILE_ALLOCS is set.
pub fn start() -> Option<Profiler> {
    let path = std::env::var("RAY_PROFILE").ok().filter(|p| !p.is_empty());
    let every = std::env::var("RAY_PROFILE_ALLOCS").ok().and_then(|v| v.parse::<u64>().ok());
    if path.is_none() && every.is_none() {
        return None;
    }
    let top = std::env::var("RAY_PROFILE_TOP").ok().and_then(|v| v.parse().ok()).unwrap_or(25);
    let before = LIVE.load(Relaxed);
    // unwinding through libc from a signal handler can deadlock
    let guard = path.and_then(|path| {
        let g = pprof::ProfilerGuardBuilder::default()
            .frequency(997)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .map_err(|e| eprintln!("profile: {e}"))
            .ok()?;
        Some((g, path))
    });
    OWN.store(LIVE.load(Relaxed) - before, Relaxed);
    PEAK.store(LIVE.load(Relaxed), Relaxed);
    EVERY.store(every.unwrap_or(0), Relaxed);
    Some(Profiler { guard, top })
}

impl Profiler {
    /// Writes the flame graph and prints the heaviest functions (self time:
    /// samples where the function itself was running; total: anywhere on
    /// the stack), source lines and allocation sites.
    pub fn finish(self) {
        EVERY.store(0, Relaxed);
        if let Some((guard, path)) = &self.guard {
            match guard.report().build() {
                Ok(r) => self.time(&r, path),
                Err(e) => eprintln!("profile: {e}"),
            }
        }
        let sites = std::mem::take(&mut *SITES.lock().unwrap());
        if !sites.is_empty() {
            self.allocs(sites);
        }
    }

    fn time(&self, report: &pprof::Report, path: &str) {
        match std::fs::File::create(path) {
            Ok(f) => {
                if let Err(e) = report.flamegraph(f) {
                    eprintln!("profile: {e}");
                }
            }
            Err(e) => eprintln!("profile: {path}: {e}"),
        }
        let (mut own, mut total, mut lines) = (HashMap::new(), HashMap::new(), HashMap::new());
        let mut threads: HashMap<String, isize> = HashMap::new();
        let (mut all, mut every) = (0isize, 0isize);
        let only = std::env::var("RAY_PROFILE_THREAD").unwrap_or_default();
        for (frames, &n) in &report.data {
            let name = frames.thread_name.trim_end_matches(|c: char| c.is_ascii_digit()).trim_end().to_string();
            *threads.entry(name).or_default() += n;
            every += n;
            if !frames.thread_name.starts_with(&only) {
                continue;
            }
            all += n;
            // frames run from the sampled instruction outwards; each address
            // lists the functions inlined there, innermost first. When the
            // sampled code is this program's (library code inlined into it
            // included), the time is charged to its innermost function that
            // is not a SIMD or vector helper
            let syms: Vec<_> = frames.frames.iter().flatten().collect();
            let ours: Vec<_> = syms.iter().filter_map(|s| own_file(s).map(|f| (s, f))).collect();
            let in_ours = frames.frames.first().is_some_and(|f| f.iter().any(|s| own_file(s).is_some()));
            match ours.iter().find(|(_, f)| !HELPERS.contains(&f.as_str())).filter(|_| in_ours) {
                Some((s, file)) => {
                    *own.entry(format!("{} ({file})", short(&s.name()))).or_default() += n;
                    *lines.entry(format!("{file}:{}", s.lineno.unwrap_or(0))).or_default() += n;
                }
                None => {
                    // library code: named with the nearest caller in this program
                    let name = syms.first().map(|s| short(&s.name())).unwrap_or_default();
                    let from = ours.first().map(|(s, f)| format!(" ← {} ({f})", short(&s.name()))).unwrap_or_default();
                    *own.entry(format!("{name} (outside ray){from}")).or_default() += n;
                }
            }
            let mut seen = std::collections::HashSet::new();
            for (s, file) in &ours {
                let f = format!("{} ({file})", short(&s.name()));
                if seen.insert(f.clone()) {
                    *total.entry(f).or_default() += n;
                }
            }
        }
        let table = |title: &str, m: HashMap<String, isize>, all: isize| {
            let mut v: Vec<_> = m.into_iter().collect();
            v.sort_by(|a, b| b.1.cmp(&a.1));
            println!("profile: {title}");
            for (k, n) in v.into_iter().take(self.top) {
                println!("{:6.2}%  {k}", n as f64 * 100.0 / all.max(1) as f64);
            }
        };
        println!("profile: {every} samples, flame graph in {path}");
        table("threads", threads, every);
        if !only.is_empty() {
            println!("profile: {all} samples in threads {only}*");
        }
        table("self time", own, all);
        table("total time", total, all);
        table("self time by line", lines, all);
    }

    /// Groups the recorded allocations by the first three places in this
    /// program on their stacks.
    fn allocs(&self, sites: Vec<(std::backtrace::Backtrace, usize)>) {
        let every = sites.len();
        let mut by: HashMap<String, (usize, usize)> = HashMap::new();
        for (bt, size) in sites {
            let text = bt.to_string();
            let mut places = Vec::new();
            let mut func = "";
            for line in text.lines().map(str::trim) {
                if let Some(at) = line.strip_prefix("at ") {
                    if let Some(file) = own_path(at)
                        && !file.starts_with("prof.rs")
                    {
                        // file:line:column, without the column
                        let file = file.rsplit_once(':').map_or(file, |(f, _)| f);
                        places.push(format!("{} {file}", short(func.rsplit("::").next().unwrap_or(func))));
                    }
                } else if let Some((_, f)) = line.split_once(": ") {
                    func = f;
                }
            }
            places.dedup();
            let key = if places.is_empty() { "(outside ray)".to_string() } else { places[..places.len().min(3)].join(" ← ") };
            let e = by.entry(key).or_default();
            e.0 += 1;
            e.1 += size;
        }
        let mut v: Vec<_> = by.into_iter().collect();
        v.sort_by(|a, b| b.1.0.cmp(&a.1.0));
        println!("profile: allocations by place ({every} recorded; each stands for RAY_PROFILE_ALLOCS of them)");
        for (k, (n, bytes)) in v.into_iter().take(self.top) {
            println!("{:6.2}%  {n:6} allocs  {:9.1} KB  {k}", n as f64 * 100.0 / every as f64, bytes as f64 / 1e3);
        }
    }
}

/// Files of small helpers that are inlined everywhere: their time is
/// charged to their callers.
const HELPERS: &[&str] = &["simd.rs", "math.rs"];

/// The file of a symbol in this program's sources, relative to src/.
fn own_file(s: &pprof::Symbol) -> Option<String> {
    own_path(&s.filename.as_ref()?.display().to_string()).map(str::to_string)
}

/// A path in this program's sources, relative to src/.
fn own_path(f: &str) -> Option<&str> {
    let src = concat!(env!("CARGO_MANIFEST_DIR"), "/src/");
    f.strip_prefix(src).or_else(|| f.strip_prefix("./src/")).or_else(|| f.strip_prefix("src/"))
}

/// A function name without its hash.
fn short(name: &str) -> String {
    let mut s = name.to_string();
    if let Some(i) = s.rfind("::h")
        && s[i + 3..].chars().all(|c| c.is_ascii_hexdigit())
    {
        s.truncate(i);
    }
    s
}
