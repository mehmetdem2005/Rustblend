use core::f32::consts::{PI, TAU};

use crate::math::{Mat4, Vec3};

const MIN_DISTANCE: f32 = 0.05;
const MAX_DISTANCE: f32 = 1_000_000.0;
const MAX_PITCH: f32 = PI * 0.5 - 0.001;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CameraError {
    InvalidTarget,
    InvalidDistance,
    InvalidAspect,
    InvalidViewport,
    InvalidInput,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Camera {
    target: Vec3,
    yaw: f32,
    pitch: f32,
    distance: f32,
    aspect: f32,
}

impl Camera {
    // Kamera oluşturur.
    // Yaw başlangıçta 0: Kamera +Z yönünden bakar.
    pub fn new(
        target: Vec3,
        distance: f32,
        aspect: f32,
    ) -> Result<Self, CameraError> {
        if !target.is_finite() {
            return Err(CameraError::InvalidTarget);
        }

        if !distance.is_finite() || distance <= 0.0 {
            return Err(CameraError::InvalidDistance);
        }

        if !aspect.is_finite() || aspect <= 0.0 {
            return Err(CameraError::InvalidAspect);
        }

        Ok(Self {
            target,
            yaw: 0.0,
            pitch: 0.0,
            distance: distance.clamp(
                MIN_DISTANCE,
                MAX_DISTANCE,
            ),
            aspect,
        })
    }

    pub fn orbit_angles(&self) -> (f32, f32) { (self.yaw, self.pitch) }

    pub fn target(&self) -> Vec3 {
        self.target
    }

    pub fn distance(&self) -> f32 {
        self.distance
    }

    pub fn aspect(&self) -> f32 {
        self.aspect
    }

    // Kameranın dünyadaki konumu.
    pub fn position(&self) -> Vec3 {
        let cos_pitch = self.pitch.cos();

        self.target + Vec3::new(
            self.distance * cos_pitch * self.yaw.sin(),
            self.distance * self.pitch.sin(),
            self.distance * cos_pitch * self.yaw.cos(),
        )
    }

    // Kamerayı hedefin etrafında döndürür.
    // Açılar radyan cinsindendir.
    pub fn orbit(
        &mut self,
        delta_yaw: f32,
        delta_pitch: f32,
    ) -> Result<(), CameraError> {
        if !delta_yaw.is_finite()
            || !delta_pitch.is_finite()
        {
            return Err(CameraError::InvalidInput);
        }

        self.yaw = (self.yaw + delta_yaw).rem_euclid(TAU);

        self.pitch = (self.pitch + delta_pitch).clamp(
            -MAX_PITCH,
            MAX_PITCH,
        );

        Ok(())
    }

    // factor < 1: yakınlaştırır.
    // factor > 1: uzaklaştırır.
    pub fn zoom(
        &mut self,
        factor: f32,
    ) -> Result<(), CameraError> {
        if !factor.is_finite() || factor <= 0.0 {
            return Err(CameraError::InvalidInput);
        }

        self.distance = (self.distance * factor).clamp(
            MIN_DISTANCE,
            MAX_DISTANCE,
        );

        Ok(())
    }

    // Kameranın baktığı alanı yatay ve dikey kaydırır.
    pub fn pan(
        &mut self,
        horizontal: f32,
        vertical: f32,
    ) -> Result<(), CameraError> {
        if !horizontal.is_finite()
            || !vertical.is_finite()
        {
            return Err(CameraError::InvalidInput);
        }

        let forward = (self.target - self.position())
            .normalized()
            .expect("Camera direction must be valid");

        let world_up = Vec3::new(0.0, 1.0, 0.0);

        let right = forward.cross(world_up)
            .normalized()
            .expect("Camera right direction must be valid");

        let up = right.cross(forward)
            .normalized()
            .expect("Camera up direction must be valid");

        let new_target = self.target
            + right * horizontal
            + up * vertical;

        if !new_target.is_finite() {
            return Err(CameraError::InvalidInput);
        }

        self.target = new_target;

        Ok(())
    }

    // Android ekranı döndüğünde çağrılır.
    pub fn set_viewport(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<(), CameraError> {
        if width == 0 || height == 0 {
            return Err(CameraError::InvalidViewport);
        }

        self.aspect = width as f32 / height as f32;

        Ok(())
    }

    // Kamera dünyasını kamera koordinatlarına dönüştürür.
    pub fn view_matrix(&self) -> Mat4 {
        let position = self.position();

        let forward = (self.target - position)
            .normalized()
            .expect("Camera direction must be valid");

        let world_up = Vec3::new(0.0, 1.0, 0.0);

        let right = forward.cross(world_up)
            .normalized()
            .expect("Camera right direction must be valid");

        let up = right.cross(forward)
            .normalized()
            .expect("Camera up direction must be valid");

        Mat4 {
            elements: [
                right.x,
                up.x,
                -forward.x,
                0.0,

                right.y,
                up.y,
                -forward.y,
                0.0,

                right.z,
                up.z,
                -forward.z,
                0.0,

                -right.dot(position),
                -up.dot(position),
                forward.dot(position),
                1.0,
            ],
        }
    }

    // Perspektif projeksiyon matrisi.
    pub fn projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh_zo(
            PI / 3.0,
            self.aspect,
            0.1,
            1000.0,
        )
        .expect("Camera projection parameters must be valid")
    }

    // Dünya -> Kamera -> Ekran dönüşümü için birleşik matris.
    pub fn view_projection_matrix(&self) -> Mat4 {
        self.projection_matrix() * self.view_matrix()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn camera() -> Camera {
        Camera::new(Vec3::ZERO, 5.0, 1.0).unwrap()
    }

    #[test]
    fn initial_camera_position() {
        let c = camera();

        assert_eq!(
            c.position(),
            Vec3::new(0.0, 0.0, 5.0)
        );
    }

    #[test]
    fn view_places_camera_at_origin() {
        let c = camera();
        let result = c.view_matrix()
            .transform_point(c.position())
            .unwrap();

        assert!(result.length() < 0.00001);
    }

    #[test]
    fn view_places_target_in_front() {
        let c = camera();

        let result = c.view_matrix()
            .transform_point(c.target())
            .unwrap();

        assert!(result.x.abs() < 0.00001);
        assert!(result.y.abs() < 0.00001);
        assert!((result.z + 5.0).abs() < 0.00001);
    }

    #[test]
    fn orbit_rotates_around_target() {
        let mut c = camera();

        c.orbit(PI / 2.0, 0.0).unwrap();

        let p = c.position();

        assert!((p.x - 5.0).abs() < 0.00001);
        assert!(p.z.abs() < 0.00001);
    }

    #[test]
    fn pitch_is_clamped() {
        let mut c = camera();

        c.orbit(0.0, 10.0).unwrap();

        let p = c.position();

        assert!(p.y < 5.0);
        assert!(p.y > 4.9);
    }

    #[test]
    fn zoom_changes_distance() {
        let mut c = camera();

        c.zoom(0.5).unwrap();

        assert!((c.distance() - 2.5).abs() < 0.00001);
    }

    #[test]
    fn zoom_rejects_invalid_factor() {
        let mut c = camera();

        assert_eq!(
            c.zoom(0.0),
            Err(CameraError::InvalidInput)
        );

        assert_eq!(
            c.zoom(f32::NAN),
            Err(CameraError::InvalidInput)
        );
    }

    #[test]
    fn pan_moves_target() {
        let mut c = camera();

        c.pan(1.0, 2.0).unwrap();

        assert_eq!(
            c.target(),
            Vec3::new(1.0, 2.0, 0.0)
        );
    }

    #[test]
    fn viewport_updates_aspect() {
        let mut c = camera();

        c.set_viewport(1920, 1080).unwrap();

        assert!((c.aspect() - 16.0 / 9.0).abs() < 0.00001);

        c.set_viewport(1080, 1920).unwrap();

        assert!((c.aspect() - 9.0 / 16.0).abs() < 0.00001);
    }

    #[test]
    fn viewport_rejects_zero_dimensions() {
        let mut c = camera();

        assert_eq!(
            c.set_viewport(0, 1080),
            Err(CameraError::InvalidViewport)
        );

        assert_eq!(
            c.set_viewport(1920, 0),
            Err(CameraError::InvalidViewport)
        );
    }

    #[test]
    fn projection_matrix_is_finite() {
        let c = camera();

        let matrix = c.view_projection_matrix();

        assert!(
            matrix.elements.iter().all(|v| v.is_finite())
        );
    }

    #[test]
    fn constructor_rejects_invalid_distance() {
        assert_eq!(
            Camera::new(Vec3::ZERO, 0.0, 1.0),
            Err(CameraError::InvalidDistance)
        );
    }

    #[test]
    fn constructor_rejects_invalid_target() {
        assert_eq!(
            Camera::new(
                Vec3::new(f32::NAN, 0.0, 0.0),
                5.0,
                1.0
            ),
            Err(CameraError::InvalidTarget)
        );
    }
}
