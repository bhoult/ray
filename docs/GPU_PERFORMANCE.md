# GPU performance: what to try, in order

A plan, from reading `src/gpu/` and the numbers in the README. Steps 1 and 2 are built and measured ([below](#measured)); steps 3 and 4 are not yet: the profile needs Nsight Graphics, which the machine they were measured on does not have. Each step says how to tell whether it worked, and the later steps depend on what the earlier ones show.

In short: two cheap changes first (wgpu's runtime checks off, per-mode kernels built in the background), then a profile, and only then a wavefront renderer, split at bounces, if the profile says the time goes where a wavefront design helps. The ray-tracing hardware and CUDA are left out, for the reasons at the end.

## Where the GPU loses time

The GPU renderer is one compute kernel with one thread a pixel (`main` in `src/gpu/lib.wgsl`, which calls `render_sample` for each sample). A GPU runs threads in groups of 32 that step through the same instructions together, so three things slow a kernel like this down, and they call for different fixes:

- **The spread of step counts.** One ray in a group marches 20 steps and its neighbour 300, and the whole group waits for the slowest. This is most of the cost of a scene with one large fractal. Sorting rays by what they hit does nothing for it; the fix is to march in chunks and refill the lanes of rays that have finished. The search-steps view (`--mode steps`, key 6) shows it already: a wide spread within an 8 x 8 tile is the cost.
- **Different code paths.** Rays in a group run different shapes' distance functions, different materials or different branches of the integrator, and the group runs each path in turn. This is most of the cost after the first bounce in path-traced mode, in glass and chrome, and in `fractal-catalog`. A wavefront design fixes this.
- **Registers.** One large kernel reserves registers for its most demanding path, so fewer groups fit on the GPU at once and it hides memory latency less well. The README has measured this: building the kernel per mode and lighting flags was 12-15% faster.

## 1. wgpu's runtime checks off (done: the loop counters)

`src/gpu/mod.rs` builds the shader with `create_shader_module`, which in wgpu 27 means `ShaderRuntimeChecks::checked()`:

- **Bounds checks:** where the platform can't bound buffer access itself, wgpu adds a clamp to every buffer access, and the shader reads the scene buffer constantly.
- **Loop bounding:** wgpu adds a counter and a compare to every loop, so that a loop that never ends can't hang the driver. The renderer is mostly loops.

`unsafe { device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked()) }` turns both off. It is safe here only if every loop in the shader has a hard bound and no index can run past its buffer, including for a bad scene file; check both before keeping it. On Vulkan the driver may already bound buffer access itself (robust buffer access), in which case wgpu adds no clamps of its own and the gain is mostly the loop counters. It may be small; it is a one-line A/B.

How to tell: `--bench --device gpu` on `showcase`, `fractals`, `stress-1600` and `nurbs` before and after (medians), and `tools/parity.sh` unchanged.

**Done.** The loop counters are off (`CHECKS` in `src/gpu/mod.rs`); the bounds checks stay. On the RTX 5080 the driver bounds buffer reads itself (Vulkan's robust buffer access 2), so wgpu adds no clamps to them anyway, and turning off the ones it still adds (to indexes into local arrays) measured no faster; on a platform that does not bound buffer reads, wgpu's clamps keep doing it. Every `for` loop in `lib.wgsl` counts up to a bound, and the five BVH walks, which end because a walk over a tree pops each node once at most, also stop after `MAX_VISITS` nodes, so every loop has a hard bound. The counters cost most where loops nest deepest: `nurbs-booleans`, whose NURBS leaves run a Newton solve inside every distance evaluation, takes a third to a half less time without them, and the path-traced `showcase` a fifth less. `nurbs` is about 9% slower in Direct mode.

## 2. Per-mode kernels, built in the background (done)

The 12-15% per-mode kernels were left out because a new combination took a 6-second build. The viewer already has the pattern to avoid that: at startup the CPU draws while the GPU's shader builds. Do the same here: keep the general kernel drawing, build the variant for the current mode and lighting flags on a thread, and swap when it is ready. The driver's cache (`~/.cache/nvidia`) makes a combination it has seen cost milliseconds.

How to tell: the same benches, and no frame held up while a variant builds.

**Done.** The kernel reads its mode and lighting flags through two pipeline-overridable constants, `SPEC_MODE` and `SPEC_FLAGS` (`lib.wgsl`). Left at their defaults, the general kernel reads them from the frame; set, the driver drops the code they turn off. The viewer builds the general kernel and the one for the current mode and flags at the same time, each on its own thread. It draws with whichever is ready first and switches to the one for the mode when it arrives; its stderr says which was built when. A new mode or flag is drawn by the general kernel at once while its own kernel builds, and the kernels built stay until the scene's structures change. A headless render builds only the kernel for its mode. `RAY_GENERAL_KERNEL=1` keeps the general kernel, to compare the two. The two draw the same picture: `tools/parity.sh` matched main's images exactly in 22 of 23 scenes with either kernel, and the 23rd to within a millionth on average.

With the kernels, several scenes draw 5-55% faster, the others about as before, and `nurbs` about 10% slower, in both modes. Building it with the mode alone or the flags alone did not help path tracing. Why is open; the profile in step 3 would show whether the driver needs more registers for it once the mode is known.

### Measured

2026-11-06, RTX 5080, 1280 x 720, `--bench --device gpu`: the GPU's time per frame, the median of three runs of 25 frames, with main and the two kernels interleaved. Other programs used 5-20% of the GPU, and runs of one build differ by up to about 10%, so differences that small are noise.

| scene | mode | main | loop counters off, general kernel | and per-mode kernels |
| --- | --- | --- | --- | --- |
| `showcase` | Direct | 16.6 ms | 15.4 ms (-7%) | 11.4 ms (-31%) |
| `showcase` | path traced | 33.5 ms | 26.8 ms (-20%) | 16.1 ms (-52%) |
| `stress-1600` | Direct | 11.8 ms | 11.6 ms | 8.7 ms (-26%) |
| `stress-1600` | path traced | 5.6 ms | 5.5 ms | 5.3 ms (-5%) |
| `fractals` | Direct | 33.1 ms | 33.5 ms | 30.2 ms (-9%) |
| `fractals` | path traced | 31.3 ms | 31.3 ms | 31.0 ms |
| `glass` | Direct | 38.8 ms | 36.1 ms | 40.5 ms (+4%) |
| `glass` | path traced | 27.8 ms | 27.6 ms | 28.4 ms (+2%) |
| `bonsai` | Direct | 69.6 ms | 71.8 ms | 67.6 ms (-3%) |
| `bonsai` | path traced | 64.4 ms | 60.7 ms (-6%) | 57.3 ms (-11%) |
| `nurbs` | Direct | 21.4 ms | 23.3 ms (+9%) | 23.6 ms (+10%) |
| `nurbs` | path traced | 32.6 ms | 32.1 ms | 36.4 ms (+12%) |
| `nurbs-booleans`* | Direct | 1,662 ms | 926 ms (-44%) | 900 ms (-46%) |
| `nurbs-booleans`* | path traced | 2,911 ms | 1,942 ms (-33%) | 1,316 ms (-55%) |

\* From an earlier run of two repeats, in which this branch's general kernel with wgpu's checks all on stood in for main.

## 3. Profile before restructuring anything

- **Nsight Graphics, GPU Trace** works on Vulkan compute (no CUDA needed). It shows how full the GPU is (occupancy) and how many threads of each group are doing useful work (warp issue efficiency, active threads per warp).
- **The search-steps view** for the spread of step counts.

Then:

| what the profile shows | what to do |
| --- | --- |
| low occupancy, registers the limit | step 2 helps; smaller kernels (wavefront) help more |
| a wide spread of step counts, few active threads in fractal scenes | march in chunks and refill finished lanes; wavefront doesn't help |
| few active threads in path-traced, glass and many-material scenes | wavefront (step 4) |
| none of these | leave the kernel as it is |

## 4. Wavefront, split at bounces (if step 3 says so)

The reference is Laine, Karras and Aila, "Megakernels Considered Harmful: Wavefront Path Tracing on GPUs" (HPG 2013). Its gains are largest for path tracing with many materials, and small for simple scenes.

A small version, rather than one kernel per material:

- One kernel finds each ray's first hit and writes the ray's state (origin, direction, throughput, random state, pixel, bounce, medium) to a storage buffer.
- One sorts and compacts by what the ray hit: the object id (the kernel already writes it out) and the material class. Finished rays drop out.
- One shades and makes the next ray (and the shadow rays), and the loop goes round again for each bounce, with `dispatch_workgroups_indirect` sized by atomic counters, so no read-back between bounces.
- The present kernel stays as the reference, for `tools/parity.sh` and as a fallback.

What it costs:

- **The port no longer has the CPU renderer's shape.** Today `lib.wgsl` is the CPU renderer ported per pixel, which is what keeps the two in agreement. A wavefront GPU renderer is structured differently, so every new feature is a different port, and the parity tests have to cover the new pipeline. That cost is paid on every feature, not once.
- **Memory traffic:** each ray's state is written and read at every stage (tens of bytes a ray each time).
- **More dispatches a frame:** the GPU's fixed cost per frame grows, which matters most on small frames.

Where it would pay: at 2560 x 1440 the GPU alone takes 45 ms and GPU + CPU 43 ms, so a faster GPU speeds up nearly the whole frame. At 640 x 360 the fixed cost of submitting and copying back leaves little to win.

## The ray-tracing hardware and CUDA

Neither is planned. What each would give, and the route that doesn't need CUDA:

| what | with CUDA | without |
| --- | --- | --- |
| profiling: why threads stall, cost per line | Nsight Compute | Nsight Graphics on Vulkan covers most of it |
| reordering rays by what they hit, in hardware (most of wavefront's gain without restructuring) | OptiX's `optixReorder` | `VK_NV_ray_tracing_invocation_reorder`, through `ash`, in a Vulkan ray-tracing pipeline (not a compute shader); NVIDIA only either way |
| RT cores for the walk over objects' bounding boxes | OptiX, with custom intersection programs | `VK_KHR_ray_query` with AABB geometry and the hit tested in the shader, through `ash`. wgpu 27's ray query takes triangle geometry only (`BlasGeometries::TriangleGeometries`) |
| a denoiser for path-traced mode | OptiX's | Intel Open Image Denoise, on any vendor; it takes images in host memory, and the GPU's samples already come back to the CPU |

The RT cores do two tests in hardware: a ray against a box and a ray against a triangle. Here that would only help the walk over objects' bounding boxes, which the README's numbers suggest matters when there are many objects (the GPU's share of the rows falls from about 90% to 35% as they grow). For fractals it does nothing: the march is the cost, and no fixed-function unit does a march, or the NURBS surfaces' Newton solves.

CUDA on its own, the same renderer written in CUDA instead of WGSL, would run on the same hardware through the same NVIDIA compiler backend, so it would be about as fast. It would also make the GPU renderer NVIDIA only (today it runs on AMD and Intel through Vulkan or GL), give `codegen.rs` a second language to write, and keep the multi-second builds (NVRTC compiles a large kernel slowly too).
