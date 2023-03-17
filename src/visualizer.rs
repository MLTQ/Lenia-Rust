// visualizer.rs

use crate::lenia::{apply_kernel, KernelParams};
use ndarray::Array2;
use pixels::{Error, Pixels, SurfaceTexture};
use winit::dpi::LogicalSize;
use winit::event::{Event, VirtualKeyCode, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;
use std::thread::sleep;
use std::time::Duration;

pub fn run_visualization(
    kernel_size: usize,
    decay_constant: f64,
    decay_type: String,
    penalty_constant: f64,
    refresh_rate: u64,
    initial_state: Vec<Vec<f64>>,
) {
    let event_loop = EventLoop::new();
    let window_size = LogicalSize::new(initial_state[0].len() as f64, initial_state.len() as f64);

    let window = WindowBuilder::new()
        .with_title("Lenia")
        .with_inner_size(window_size)
        .build(&event_loop)
        .unwrap();

    let surface_texture = SurfaceTexture::new(window.inner_size().width, window.inner_size().height, &window);
    let mut pixels = Pixels::new(window.inner_size().width, window.inner_size().height, surface_texture).unwrap();
    let world_shape = (initial_state.len(), initial_state[0].len());
    let initial_state_flat: Vec<f64> = initial_state.into_iter().flatten().collect();
    let mut world = Array2::from_shape_vec(world_shape, initial_state_flat).unwrap();


    let kernel_params = KernelParams {
        size: kernel_size,
        decay_constant,
        decay_type,
        penalty_constant,
    };

    event_loop.run(move |event, _, control_flow| {
        match event {
            Event::RedrawRequested(_) => {
                draw(&world, pixels.get_frame());
                // Add the sleep function to control the frame rate
                sleep(Duration::from_millis(1000 / refresh_rate));
                if pixels.render().is_err() {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
            }
            Event::MainEventsCleared => {
                window.request_redraw();
                world = apply_kernel(&kernel_params, &world);
            }
            Event::WindowEvent { event, .. } => match event {
                WindowEvent::CloseRequested
                | WindowEvent::KeyboardInput {
                    input:
                        winit::event::KeyboardInput {
                            virtual_keycode: Some(VirtualKeyCode::Escape),
                            ..
                        },
                    ..
                } => {
                    *control_flow = ControlFlow::Exit;
                    return;
                }
                _ => {}
            },
            _ => {}
        }
    });
}

fn draw(world: &Array2<f64>, frame: &mut [u8]) {
    let world_height = world.dim().0;
    let world_width = world.dim().1;
    let frame_height = frame.len() / (world_width * 4);
    let frame_width = world_width;

    let scale_y = world_height as f64 / frame_height as f64;
    let scale_x = world_width as f64 / frame_width as f64;

    for y in 0..frame_height {
        for x in 0..frame_width {
            let i = y * frame_width + x;
            let pixel = &mut frame[i * 4..(i + 1) * 4];

            let world_y = (y as f64 * scale_y).floor() as usize;
            let world_x = (x as f64 * scale_x).floor() as usize;

            let value = (world[[world_y, world_x]] * 255.0) as u8;


            pixel[0] = value;
            pixel[1] = value;
            pixel[2] = value;
            pixel[3] = value;
        }
    }
}
