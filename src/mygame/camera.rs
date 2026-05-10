use bytemuck::{Pod, Zeroable};
use cgmath::{InnerSpace, Matrix3, Matrix4, Point3, SquareMatrix, Vector3};
use winit::{
    event::{DeviceEvent, KeyEvent, WindowEvent},
    keyboard::KeyCode,
};

pub struct Camera {
    eye: Point3<f32>,
    direction: Vector3<f32>,
    up: Vector3<f32>,
}

#[allow(dead_code)]
impl Camera {
    pub fn new() -> Self {
        Self {
            eye: Point3::new(0.0, 0.0, 0.0),
            direction: Vector3::unit_z(),
            up: Vector3::unit_y(),
        }
    }

    pub fn up(&self) -> Vector3<f32> {
        self.up
    }

    pub fn right(&self) -> Vector3<f32> {
        self.up().cross(self.direction)
    }

    pub fn view(&self) -> Matrix4<f32> {
        cgmath::Matrix4::look_to_lh(self.eye, self.direction, self.up())
    }

    pub fn uniform(&self) -> CameraUniform {
        CameraUniform {
            view: self.view().into(),
            inverse_view: self.view().invert().unwrap().into(),
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Zeroable, Pod)]
pub struct CameraUniform {
    view: [[f32; 4]; 4],
    inverse_view: [[f32; 4]; 4],
}

pub struct Axis {
    negative_pressed: bool,
    negative_button: KeyCode,
    positive_pressed: bool,
    positive_button: KeyCode,
}

impl Axis {
    pub fn new(negative_button: KeyCode, positive_button: KeyCode) -> Self {
        Self {
            negative_button,
            positive_button,
            negative_pressed: false,
            positive_pressed: false,
        }
    }

    pub fn process(&mut self, key_event: &KeyEvent) {
        if key_event.physical_key == self.positive_button {
            self.positive_pressed = key_event.state.is_pressed();
        } else if key_event.physical_key == self.negative_button {
            self.negative_pressed = key_event.state.is_pressed();
        }
    }

    pub fn get(&self) -> f32 {
        return if self.negative_pressed { -1.0 } else { 0.0 }
            + if self.positive_pressed { 1.0 } else { 0.0 };
    }
}

pub struct CameraController {
    pub speed: f32,
    pub sensitivity: f32,
    camera_motion: (f32, f32),
    horizontal: Axis,
    vertical: Axis,
    qe_axis: Axis,
}

impl CameraController {
    pub fn new(speed: f32, sensitivity: f32) -> Self {
        Self {
            speed,
            sensitivity,
            camera_motion: (0.0, 0.0),
            horizontal: Axis::new(KeyCode::KeyA, KeyCode::KeyD),
            vertical: Axis::new(KeyCode::KeyS, KeyCode::KeyW),
            qe_axis: Axis::new(KeyCode::KeyQ, KeyCode::KeyE),
        }
    }

    pub fn process_window_events(&mut self, window_event: &WindowEvent) {
        match window_event {
            WindowEvent::KeyboardInput { ref event, .. } => {
                self.horizontal.process(event);
                self.vertical.process(event);
                self.qe_axis.process(event);
            }
            WindowEvent::MouseWheel { delta, .. } => {
                match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y) => {
                        self.speed *= 1.0 + y * 0.1;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    pub fn process_device_events(&mut self, device_event: &DeviceEvent) {
        match device_event {
            DeviceEvent::MouseMotion { delta } => {
                self.camera_motion.0 += delta.0 as f32;
                self.camera_motion.1 += delta.1 as f32;
            }
            _ => {}
        }
    }

    pub fn update(&mut self, camera: &mut Camera, delta: f32) {
        let roty = Matrix3::from_axis_angle(
            camera.up,
            cgmath::Rad(self.camera_motion.0 * self.sensitivity)
        );
        let rotx = Matrix3::from_axis_angle(
            camera.right(),
            cgmath::Rad(self.camera_motion.1 * self.sensitivity)
        );

        camera.direction = roty * rotx * Vector3::unit_z();

        let movement = (
                self.horizontal.get() * camera.right()
                + self.vertical.get() * camera.direction
            ).normalize()
            * self.speed
            * delta;
        
        // camera.up = Matrix3::from_axis_angle(
        //     camera.direction,
        //     cgmath::Rad(-self.qe_axis.get() * delta)
        // ) * camera.up;

        if self.horizontal.get() != 0.0 || self.vertical.get() != 0.0 {
            camera.eye += movement;
        }
    }
}
