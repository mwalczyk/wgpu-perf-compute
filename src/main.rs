use wgpu_perf_compute::run;

fn main() {
    pollster::block_on(run());
}
