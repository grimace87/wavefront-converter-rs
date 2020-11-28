use std::io::Write;
use std::fs::File;
use std::fmt::Debug;

use crate::modelfactory::FILE_VERSION_NUMBER;

pub const WALL_NORMAL_ELEVATION_MIN: f32 = -0.0873; // about 5 degrees
pub const WALL_NORMAL_ELEVATION_MAX: f32 = 0.0873;
pub const SLIDE_NORMAL_ELEVATION_MIN: f32 = -0.6981; // about 50 degrees
pub const SLIDE_NORMAL_ELEVATION_MAX: f32 = 0.6981;

#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Vec3 {
    pub x: f32,
    pub y: f32,
    pub z: f32
}

impl Vec3 {
    pub fn dot(&self, other: &Vec3) -> f32 {
        self.x * other.x + self.y * other.y + self.z * other.z
    }

    pub fn len(&self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }

    pub fn normalise(&self) -> Vec3 {
        let length = (self.x * self.x + self.y * self.y + self.z * self.z).sqrt();
        if length == 0.0 {
            Vec3::default()
        } else {
            Vec3 {
                x: self.x / length,
                y: self.y / length,
                z: self.z / length
            }
        }
    }
}

impl std::ops::Add<Vec3> for Vec3 {
    type Output = Vec3;

    fn add(self, rhs: Vec3) -> Self::Output {
        Vec3 { x: self.x + rhs.x, y: self.y + rhs.y, z: self.z + rhs.z }
    }
}

impl std::ops::Sub<Vec3> for Vec3 {
    type Output = Vec3;

    fn sub(self, rhs: Vec3) -> Self::Output {
        Vec3 { x: self.x - rhs.x, y: self.y - rhs.y, z: self.z - rhs.z }
    }
}

impl std::ops::Mul<f32> for Vec3 {
    type Output = Vec3;

    fn mul(self, rhs: f32) -> Self::Output {
        Vec3 { x: self.x * rhs, y: self.y * rhs, z: self.z * rhs }
    }
}

/// Surfaces are triangles, with a normal stored alongside vertices for convenience
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Surface {
    pub point_0: Vec3,
    pub point_1: Vec3,
    pub point_2: Vec3,
    pub normal: Vec3
}

/// Walls are defined by 2 points which specify opposite corners of a rectangle, plus a normal for
/// convenience
#[repr(C)]
#[derive(Default, Clone, Copy, Debug, PartialEq)]
pub struct Wall {
    bottom_left: Vec3,
    top_right: Vec3,
    normal: Vec3
}

impl Wall {
    pub fn from_bottom_left_to_top_right(bottom_left: Vec3, top_right: Vec3) -> Wall {
        let bottom_edge = Vec3 {
            x: top_right.x - bottom_left.x,
            y: 0.0,
            z: top_right.z - bottom_left.z
        };
        let normal_direction = Vec3 {
            x: -bottom_edge.z,
            y: 0.0,
            z: bottom_edge.x
        };
        Wall {
            bottom_left,
            top_right,
            normal: normal_direction.normalise()
        }
    }
}

pub struct CollisionData {
    model_name: String,
    pub extent_x: [f32; 2],
    pub extent_y: [f32; 2],
    pub extent_z: [f32; 2],
    pub traction_surfaces: Vec<Surface>,
    pub sliding_surfaces: Vec<Surface>,
    pub walls: Vec<Wall>
}

impl CollisionData {

    pub fn new(model_name: String) -> CollisionData {
        CollisionData {
            model_name: model_name,
            extent_x: [0.0, 0.0],
            extent_y: [0.0, 0.0],
            extent_z: [0.0, 0.0],
            traction_surfaces: vec![],
            sliding_surfaces: vec![],
            walls: vec![]
        }
    }

    pub fn get_model_name(&self) -> &String {
        &self.model_name
    }

    pub fn remove_wall_duplicates(&mut self) {
        if self.walls.len() < 2 {
            return;
        }
        let mut indices_to_remove: Vec<usize> = vec![];
        for index in 0..(self.walls.len() - 1) {
            let wall_being_searched = &self.walls[index];
            for scan_index in (index + 1)..self.walls.len() {
                let wall_to_compare_with = &self.walls[scan_index];
                if (wall_being_searched.bottom_left.y - wall_to_compare_with.bottom_left.y).abs() > 0.01 {
                    continue;
                }
                if (wall_being_searched.top_right.y - wall_to_compare_with.top_right.y).abs() > 0.01 {
                    continue;
                }
                let mut left_to_left = wall_being_searched.bottom_left - wall_to_compare_with.bottom_left;
                left_to_left.y = 0.0;
                if left_to_left.len() > 0.01 {
                    continue;
                }
                let mut right_to_right = wall_being_searched.top_right - wall_to_compare_with.top_right;
                right_to_right.y = 0.0;
                if right_to_right.len() > 0.01 {
                    continue;
                }
                indices_to_remove.push(index);
            }
        }

        indices_to_remove.reverse();
        for index in indices_to_remove {
            self.walls.remove(index);
        }
    }

    pub fn find_extents(&mut self) {
        let mut x_min = 0f32;
        let mut x_max = 0f32;
        let mut y_min = 0f32;
        let mut y_max = 0f32;
        let mut z_min = 0f32;
        let mut z_max = 0f32;

        for surface in self.traction_surfaces.iter() {
            for point in [&surface.point_0, &surface.point_1, &surface.point_2].iter() {
                if point.x < x_min {
                    x_min = point.x;
                }
                if point.x > x_max {
                    x_max = point.x;
                }
                if point.y < y_min {
                    y_min = point.y;
                }
                if point.y > y_max {
                    y_max = point.y;
                }
                if point.z < z_min {
                    z_min = point.z;
                }
                if point.z > z_max {
                    z_max = point.z;
                }
            }
        }

        for surface in self.sliding_surfaces.iter() {
            for point in [&surface.point_0, &surface.point_1, &surface.point_2].iter() {
                if point.x < x_min {
                    x_min = point.x;
                }
                if point.x > x_max {
                    x_max = point.x;
                }
                if point.y < y_min {
                    y_min = point.y;
                }
                if point.y > y_max {
                    y_max = point.y;
                }
                if point.z < z_min {
                    z_min = point.z;
                }
                if point.z > z_max {
                    z_max = point.z;
                }
            }
        }

        for wall in self.walls.iter() {
            for point in [&wall.bottom_left, &wall.bottom_left].iter() {
                if point.x < x_min {
                    x_min = point.x;
                }
                if point.x > x_max {
                    x_max = point.x;
                }
                if point.y < y_min {
                    y_min = point.y;
                }
                if point.y > y_max {
                    y_max = point.y;
                }
                if point.z < z_min {
                    z_min = point.z;
                }
                if point.z > z_max {
                    z_max = point.z;
                }
            }
        }

        self.extent_x[0] = x_min;
        self.extent_x[1] = x_max;
        self.extent_y[0] = y_min;
        self.extent_y[1] = y_max;
        self.extent_z[0] = z_min;
        self.extent_z[1] = z_max;
    }

    /// # Safety
    /// Should be safe to use - current self should have well-formed data Vecs
    pub unsafe fn write_data_to_file(&self, file: &mut File) -> std::io::Result<()> {
        file.write_all(&FILE_VERSION_NUMBER.to_ne_bytes())?;
        file.write_all(&self.extent_x[0].to_ne_bytes())?;
        file.write_all(&self.extent_x[1].to_ne_bytes())?;
        file.write_all(&self.extent_y[0].to_ne_bytes())?;
        file.write_all(&self.extent_y[1].to_ne_bytes())?;
        file.write_all(&self.extent_z[0].to_ne_bytes())?;
        file.write_all(&self.extent_z[1].to_ne_bytes())?;

        let surface_count = self.traction_surfaces.len() as u32;
        file.write_all(&surface_count.to_ne_bytes())?;
        assert_eq!(std::mem::size_of::<Surface>(), 48);
        for surface in self.traction_surfaces.iter() {
            file.write_all(&*(surface as *const Surface as *const [u8; 48]))?;
        }

        let surface_count = self.sliding_surfaces.len() as u32;
        file.write_all(&surface_count.to_ne_bytes())?;
        assert_eq!(std::mem::size_of::<Surface>(), 48);
        for surface in self.sliding_surfaces.iter() {
            file.write_all(&*(surface as *const Surface as *const [u8; 48]))?;
        }

        let surface_count = self.walls.len() as u32;
        file.write_all(&surface_count.to_ne_bytes())?;
        assert_eq!(std::mem::size_of::<Wall>(), 36);
        for surface in self.walls.iter() {
            file.write_all(&*(surface as *const Wall as *const [u8; 36]))?;
        }

        Ok(())
    }

    /// # Safety
    /// Should be safe if processing files generated with the same version of this tool
    pub unsafe fn from_bytes(bytes: &[u8]) -> CollisionData {

        let version_ptr = bytes.as_ptr();
        let version_number = *(version_ptr as *const u32);
        if version_number != FILE_VERSION_NUMBER {
            panic!("Bad file version: expected {} but was {}", FILE_VERSION_NUMBER, version_number);
        }

        let extent_ptr = version_ptr.add(4);
        let extent_x_min = *(extent_ptr as *const f32);
        let extent_ptr = extent_ptr.add(4);
        let extent_x_max = *(extent_ptr as *const f32);
        let extent_ptr = extent_ptr.add(4);
        let extent_y_min = *(extent_ptr as *const f32);
        let extent_ptr = extent_ptr.add(4);
        let extent_y_max = *(extent_ptr as *const f32);
        let extent_ptr = extent_ptr.add(4);
        let extent_z_min = *(extent_ptr as *const f32);
        let extent_ptr = extent_ptr.add(4);
        let extent_z_max = *(extent_ptr as *const f32);

        let surface_count_ptr = extent_ptr.add(4);
        let surface_count = *(surface_count_ptr as *const u32);
        let mut traction_surfaces: Vec<Surface> = vec![Surface::default(); surface_count as usize];
        let surfaces_ptr = surface_count_ptr.add(4);
        let surface_slice = std::slice::from_raw_parts(surfaces_ptr as *const Surface, surface_count as usize);
        traction_surfaces.copy_from_slice(surface_slice);

        let surface_count_ptr = surfaces_ptr.add(surface_count as usize * std::mem::size_of::<Surface>());
        let surface_count = *(surface_count_ptr as *const u32);
        let mut sliding_surfaces: Vec<Surface> = vec![Surface::default(); surface_count as usize];
        let surfaces_ptr = surface_count_ptr.add(4);
        let surface_slice = std::slice::from_raw_parts(surfaces_ptr as *const Surface, surface_count as usize);
        sliding_surfaces.copy_from_slice(surface_slice);

        let wall_count_ptr = surfaces_ptr.add(surface_count as usize * std::mem::size_of::<Surface>());
        let wall_count = *(wall_count_ptr as *const u32);
        let mut walls: Vec<Wall> = vec![Wall::default(); wall_count as usize];
        let walls_ptr = wall_count_ptr.add(4);
        let walls_slice = std::slice::from_raw_parts(walls_ptr as *const Wall, wall_count as usize);
        walls.copy_from_slice(walls_slice);

        CollisionData {
            model_name: String::from(""),
            extent_x: [extent_x_min, extent_x_max],
            extent_y: [extent_y_min, extent_y_max],
            extent_z: [extent_z_min, extent_z_max],
            traction_surfaces,
            sliding_surfaces,
            walls
        }
    }
}
