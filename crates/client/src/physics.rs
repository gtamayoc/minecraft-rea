//! Minecraft 1.8.9 Vanilla Physics engine.
//!
//! Implements 20 TPS physics tick, AABB collision resolution,
//! vanilla gravity (0.08 blocks/tick^2), vertical drag (0.98), and ground friction.

use glam::DVec3;
use minecraft_world::{Aabb, World};

/// Constants matching Minecraft 1.8.9 vanilla physics.
pub mod constants {
    /// Gravity acceleration per tick downwards (blocks/tick^2).
    pub const GRAVITY: f64 = 0.08;
    /// Drag factor applied to vertical velocity each tick.
    pub const VERTICAL_DRAG: f64 = 0.98;
    /// Horizontal drag when in air.
    pub const AIR_HORIZONTAL_DRAG: f64 = 0.91;
    /// Horizontal drag when on ground (standard blocks).
    pub const GROUND_HORIZONTAL_DRAG: f64 = 0.6 * 0.91;
    /// Jump upward velocity in blocks/tick.
    pub const JUMP_VELOCITY: f64 = 0.42;
    /// Walking speed acceleration factor.
    pub const WALK_ACCELERATION: f64 = 0.1;
}

/// Player physics state.
#[derive(Debug, Clone, PartialEq)]
pub struct PhysicsEntity {
    /// Current position in world space.
    pub position: DVec3,
    /// Current velocity in blocks/tick.
    pub velocity: DVec3,
    /// Yaw angle in degrees.
    pub yaw: f32,
    /// Pitch angle in degrees.
    pub pitch: f32,
    /// Whether the entity is currently resting on the ground.
    pub on_ground: bool,
}

impl PhysicsEntity {
    /// Creates a new physics entity at a given position.
    pub fn new(position: DVec3) -> Self {
        Self {
            position,
            velocity: DVec3::ZERO,
            yaw: 0.0,
            pitch: 0.0,
            on_ground: false,
        }
    }

    /// Advances the physics simulation by 1 tick (50 ms) applying inputs,
    /// resolving collisions with the world, and updating velocity.
    pub fn tick(&mut self, world: &World, forward_input: f64, strafe_input: f64, jump: bool) {
        // 1. Handle jumping
        if jump && self.on_ground {
            self.velocity.y = constants::JUMP_VELOCITY;
        }

        // 2. Apply movement acceleration based on yaw
        let mut move_vec = DVec3::new(strafe_input, 0.0, forward_input);
        if move_vec.length_squared() > 1e-4 {
            move_vec = move_vec.normalize();

            let yaw_rad = (self.yaw as f64).to_radians();
            let sin_yaw = yaw_rad.sin();
            let cos_yaw = yaw_rad.cos();

            // When yaw = 0 deg, forward moves along -Z (Minecraft convention).
            // strafe_input (+D) moves along +X: (strafe * cos, -strafe * sin)
            // forward_input (+W) moves along -Z: (-forward * sin, -forward * cos)
            let accel_x = move_vec.x * cos_yaw - move_vec.z * sin_yaw;
            let accel_z = -move_vec.z * cos_yaw - move_vec.x * sin_yaw;

            let accel = if self.on_ground {
                constants::WALK_ACCELERATION
            } else {
                constants::WALK_ACCELERATION * 0.2 // Reduced air control
            };

            self.velocity.x += accel_x * accel;
            self.velocity.z += accel_z * accel;
        }

        // 3. Apply gravity and vertical drag first (matching vanilla tick order)
        self.velocity.y -= constants::GRAVITY;
        self.velocity.y *= constants::VERTICAL_DRAG;

        // 4. Move and resolve AABB collisions against surrounding solid blocks
        self.move_with_collision(world);

        // 5. Apply horizontal ground/air friction
        let horizontal_drag = if self.on_ground {
            constants::GROUND_HORIZONTAL_DRAG
        } else {
            constants::AIR_HORIZONTAL_DRAG
        };

        self.velocity.x *= horizontal_drag;
        self.velocity.z *= horizontal_drag;

        // 6. Void boundary check: Respawn player if fallen below the void (Y < -20.0)
        if self.position.y < -20.0 {
            self.position = DVec3::new(0.5, 65.0, 0.5);
            self.velocity = DVec3::ZERO;
            self.on_ground = false;
        }
    }

    /// Moves the entity by its current velocity while resolving collisions with solid blocks.
    fn move_with_collision(&mut self, world: &World) {
        let original_move = self.velocity;
        let mut current_box = Aabb::player_box(self.position);

        // Expand query region to find potentially colliding blocks
        let query_min = (current_box.min + self.velocity.min(DVec3::ZERO)).floor();
        let query_max = (current_box.max + self.velocity.max(DVec3::ZERO)).ceil();

        let mut block_boxes = Vec::new();
        let min_x = query_min.x as i32 - 1;
        let max_x = query_max.x as i32 + 1;
        let min_y = (query_min.y as i32 - 1).max(0);
        let max_y = (query_max.y as i32 + 1).min(255);
        let min_z = query_min.z as i32 - 1;
        let max_z = query_max.z as i32 + 1;

        for bx in min_x..=max_x {
            for by in min_y..=max_y {
                for bz in min_z..=max_z {
                    let block = world.get_block(bx, by, bz);
                    if block.is_opaque_cube() {
                        block_boxes.push(Aabb::from_block(bx, by, bz));
                    }
                }
            }
        }

        // 1. Resolve Y axis first (gravity and standing on blocks)
        let mut move_y = original_move.y;
        for b in &block_boxes {
            move_y = b.calculate_y_offset(&current_box, move_y);
        }
        current_box.min.y += move_y;
        current_box.max.y += move_y;

        // 2. Resolve X axis
        let mut move_x = original_move.x;
        for b in &block_boxes {
            move_x = b.calculate_x_offset(&current_box, move_x);
        }
        current_box.min.x += move_x;
        current_box.max.x += move_x;

        // 3. Resolve Z axis
        let mut move_z = original_move.z;
        for b in &block_boxes {
            move_z = b.calculate_z_offset(&current_box, move_z);
        }

        // Check ground state: player moved down and was blocked
        self.on_ground = original_move.y < 0.0 && move_y != original_move.y;

        // Damp velocity if blocked along an axis
        if original_move.x != move_x {
            self.velocity.x = 0.0;
        }
        if original_move.y != move_y {
            self.velocity.y = 0.0;
        }
        if original_move.z != move_z {
            self.velocity.z = 0.0;
        }

        // Update player position
        self.position.x += move_x;
        self.position.y += move_y;
        self.position.z += move_z;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use minecraft_world::{BlockState, Chunk};

    #[test]
    fn player_falls_under_gravity_when_in_air() {
        let world = World::new();
        let mut player = PhysicsEntity::new(DVec3::new(0.0, 100.0, 0.0));

        player.tick(&world, 0.0, 0.0, false);

        assert!(player.position.y < 100.0);
        assert!(!player.on_ground);
    }

    #[test]
    fn player_lands_on_solid_ground() {
        let mut world = World::new();
        let mut chunk = Chunk::new(0, 0);
        let stone = BlockState::new(1, 0);
        // Floor at Y = 60
        for x in 0..16 {
            for z in 0..16 {
                chunk.set_block(x, 60, z, stone);
            }
        }
        world.insert_chunk(chunk);

        // Spawn right above floor
        let mut player = PhysicsEntity::new(DVec3::new(8.0, 61.5, 8.0));

        // Simulate 20 ticks (1 second)
        for _ in 0..20 {
            player.tick(&world, 0.0, 0.0, false);
        }

        // Player feet should rest exactly on top of Y = 60 (y = 61.0)
        assert!((player.position.y - 61.0).abs() < 1e-3);
        assert!(player.on_ground);
    }
}
