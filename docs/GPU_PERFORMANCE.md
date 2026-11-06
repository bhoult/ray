# GPU performance: what to try, in order

A plan, not a measurement. It comes from reading `src/gpu/` and the numbers in the README; none of it has been built or timed yet. Each step says how to tell whether it worked, and the later steps depend on what the earlier ones show.

In short: two cheap changes first (wgpu's runtime checks off, per-mode kernels built in the background), then a profile, and only then a wavefront renderer, split at bounces, if the profile says the time goes where a wavefront design helps. The ray-tracing hardware and CUDA are left out, for the reasons at the end.

## Where the GPU loses time

The GPU renderer is one compute kernel with one thread a pixel (`main` in `src/gpu/lib.wgsl`, which calls `render_sample` for each sample). A GPU runs threads in groups of 32 that step through the same instructions together, so three things slow a kernel like this down, and they call for different fixes:

- **The spread of step counts.** One ray in a group marches 20 steps and its neighbour 300, and the whole group waits for the slowest. This is most of the cost of a scene with one large fractal. Sorting rays by what they hit does nothing for it; the fix is to march in chunks and refill the lanes of rays that have finished. The search-steps view (`--mode steps`, key 6) shows it already: a wide spread within an 8 x 8 tile is the cost.
- **Different code paths.** Rays in a group run different shapes' distance functions, different materials or different branches of the integrator, and the group runs each path in turn. This is most of the cost after the first bounce in path-traced mode, in glass and chrome, and in `fractal-catalog`. A wavefront design fixes this.
- **Registers.** One large kernel reserves registers for its most demanding path, so fewer groups fit on the GPU at once and it hides memory latency less well. The README has measured this: building the kernel per mode and lighting flags was 12-15% faster.

## 1. wgpu's runtime checks off (an afternoon)

`src/gpu/mod.rs` builds the shader with `create_shader_module`, which in wgpu 27 means `ShaderRuntimeChecks::checked()`:

- **Bounds checks:** where the platform can't bound buffer access itself, wgpu adds a clamp to every buffer access, and the shader reads the scene buffer constantly.
- **Loop bounding:** wgpu adds a counter and a compare to every loop, so that a loop that never ends can't hang the driver. The renderer is mostly loops.

`unsafe { device.create_shader_module_trusted(desc, wgpu::ShaderRuntimeChecks::unchecked()) }` turns both off. It is safe here only if every loop in the shader has a hard bound and no index can run past its buffer, including for a bad scene file; check both before keeping it. On Vulkan the driver may already bound buffer access itself (robust buffer access), in which case wgpu adds no clamps of its own and the gain is mostly the loop counters. It may be small; it is a one-line A/B.

How to tell: `--bench --device gpu` on `showcase`, `fractals`, `stress-1600` and `nurbs` before and after (medians), and `tools/parity.sh` unchanged.

## 2. Per-mode kernels, built in the background

The 12-15% per-mode kernels were left out because a new combination took a 6-second build. The viewer already has the pattern to avoid that: at startup the CPU draws while the GPU's shader builds. Do the same here: keep the general kernel drawing, build the variant for the current mode and lighting flags on a thread, and swap when it is ready. The driver's cache (`~/.cache/nvidia`) makes a combination it has seen cost milliseconds.

How to tell: the same benches, and no frame held up while a variant builds.

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
