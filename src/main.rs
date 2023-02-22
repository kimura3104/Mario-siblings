//! A simplified implementation of the classic game "Breakout".

use bevy::{
    prelude::*,
    sprite::collide_aabb::{collide, Collision},
    sprite::MaterialMesh2dBundle,
    time::FixedTimestep,
};

// Defines the amount of time that should elapse between each physics step.
const TIME_STEP: f32 = 1.0 / 60.0;

// These constants are defined in `Transform` units.
// Using the default 2D camera they correspond 1:1 with screen pixels.
const BLOCK_SIZE: f32 = 20.0;
const MARIO_SIZE: Vec3 = Vec3::new(BLOCK_SIZE*2.0, BLOCK_SIZE*3.0, 0.0);
const GAP_BETWEEN_PADDLE_AND_FLOOR: f32 = 60.0;
const MARIO_XSPEED: f32 = 300.0;
const JUMP_SPEED: f32 = 800.0;
const GRAVITY: f32 = 50.0;

// How close can the paddle get to the wall
const PADDLE_PADDING: f32 = 10.0;

// We set the z-value of the ball to 1 so it renders on top in the case of overlapping sprites.
const MARIO_STARTING_POSITION: Vec3 = Vec3::new(0.0, -50.0, 1.0);
//const BALL_SIZE: Vec3 = Vec3::new(30.0, 30.0, 0.0);
//const BALL_SPEED: f32 = 100.0;
const INITIAL_BALL_DIRECTION: Vec2 = Vec2::new(-1.0, 0.0);

const WALL_THICKNESS: f32 = 20.0;
// x coordinates
const LEFT_WALL: f32 = -450.;
const RIGHT_WALL: f32 = 450.;
// y coordinates
const BOTTOM_WALL: f32 = BLOCK_SIZE * -12.0;
const TOP_WALL: f32 = 300.;

const WALL1: Vec2 = Vec2::new(BLOCK_SIZE * 10.0, BLOCK_SIZE * -6.0);
const WALL2: Vec2 = Vec2::new(BLOCK_SIZE * -10.0, BLOCK_SIZE * -6.0);
const WALL3: Vec2 = Vec2::new(0.0, 0.0);
const WALL4: Vec2 = Vec2::new(BLOCK_SIZE * 14.0, BLOCK_SIZE * -1.0);
const WALL5: Vec2 = Vec2::new(BLOCK_SIZE * -14.0, BLOCK_SIZE * -1.0);
const WALL6: Vec2 = Vec2::new(BLOCK_SIZE * 9.0, BLOCK_SIZE * 6.0);
const WALL7: Vec2 = Vec2::new(BLOCK_SIZE * -9.0, BLOCK_SIZE * 6.0);

const BRICK_SIZE: Vec2 = Vec2::new(10., 10.);
// These values are exact
const GAP_BETWEEN_PADDLE_AND_BRICKS: f32 = 270.0;
const GAP_BETWEEN_BRICKS: f32 = 5.0;
// These values are lower bounds, as the number of bricks is computed
const GAP_BETWEEN_BRICKS_AND_CEILING: f32 = 20.0;
const GAP_BETWEEN_BRICKS_AND_SIDES: f32 = 20.0;

const SCOREBOARD_FONT_SIZE: f32 = 40.0;
const SCOREBOARD_TEXT_PADDING: Val = Val::Px(5.0);

const BACKGROUND_COLOR: Color = Color::rgb(0.1, 0.1, 0.1);
const PACMAN_COLOR: Color = Color::rgb(0.3, 0.3, 0.7);
const BALL_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);
const BRICK_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const WALL_COLOR: Color = Color::rgb(0.8, 0.8, 0.8);
const TEXT_COLOR: Color = Color::rgb(0.5, 0.5, 1.0);
const SCORE_COLOR: Color = Color::rgb(1.0, 0.5, 0.5);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(Scoreboard { score: 0 })
        .insert_resource(ClearColor(BACKGROUND_COLOR))
        .add_startup_system(setup)
        .add_event::<CollisionEvent>()
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(TIME_STEP as f64))
                .with_system(check_for_collisions)
                .with_system(move_pacman.before(check_for_collisions))
                .with_system(move_mario_input.before(apply_velocity))
                .with_system(apply_velocity.before(check_for_collisions)),
        )
        .add_system(update_scoreboard)
        .add_system(bevy::window::close_on_esc)
        .run();
}

#[derive(Component)]
struct Paddle;

#[derive(Component)]
struct Ball;

#[derive(Component)]
struct Mario;

#[derive(Component)]
struct IsJumping{
    isjumping: bool,
}

#[derive(Component, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Component)]
struct Collider;

#[derive(Default)]
struct CollisionEvent;

#[derive(Component)]
struct Brick;

#[derive(Resource)]
struct CollisionSound(Handle<AudioSource>);

// This bundle is a collection of the components that define a "wall" in our game
#[derive(Bundle)]
struct WallBundle {
    // You can nest bundles inside of other bundles like this
    // Allowing you to compose their functionality
    sprite_bundle: SpriteBundle,
    collider: Collider,
}

/// Which side of the arena is this wall located on?
enum WallLocation {
    Left,
    Right,
    Bottom,
    Top,
    Locate1,
    Locate2,
    Locate3,
    Locate4,
    Locate5,
    Locate6,
    Locate7,
}

impl WallLocation {
    fn position(&self) -> Vec2 {
        match self {
            WallLocation::Left => Vec2::new(LEFT_WALL, 0.),
            WallLocation::Right => Vec2::new(RIGHT_WALL, 0.),
            WallLocation::Bottom => Vec2::new(0., BOTTOM_WALL),
            WallLocation::Top => Vec2::new(0., TOP_WALL),
            WallLocation::Locate1 => WALL1,
            WallLocation::Locate2 => WALL2,
            WallLocation::Locate3 => WALL3,
            WallLocation::Locate4 => WALL4,
            WallLocation::Locate5 => WALL5,
            WallLocation::Locate6 => WALL6,
            WallLocation::Locate7 => WALL7,
        }
    }

    fn size(&self) -> Vec2 {
        let arena_height = TOP_WALL - BOTTOM_WALL;
        let arena_width = RIGHT_WALL - LEFT_WALL;
        // Make sure we haven't messed up our constants
        assert!(arena_height > 0.0);
        assert!(arena_width > 0.0);

        match self {
            WallLocation::Left | WallLocation::Right => {
                Vec2::new(WALL_THICKNESS, arena_height + WALL_THICKNESS)
            }
            WallLocation::Bottom | WallLocation::Top => {
                Vec2::new(BLOCK_SIZE * 32.0, WALL_THICKNESS)
            }
            WallLocation::Locate1 | WallLocation::Locate2 => {
                Vec2::new(BLOCK_SIZE * 12.0, BLOCK_SIZE)
            }
            WallLocation::Locate3 => {
                Vec2::new(BLOCK_SIZE * 16.0, BLOCK_SIZE)
            }
            WallLocation::Locate4 | WallLocation::Locate5 => {
                Vec2::new(BLOCK_SIZE * 4.0, BLOCK_SIZE)
            }
            WallLocation::Locate6 | WallLocation::Locate7 => {
                Vec2::new(BLOCK_SIZE * 14.0, BLOCK_SIZE)
            }
        }
    }
}

impl WallBundle {
    // This "builder method" allows us to reuse logic across our wall entities,
    // making our code easier to read and less prone to bugs when we change the logic
    fn new(location: WallLocation) -> WallBundle {
        WallBundle {
            sprite_bundle: SpriteBundle {
                transform: Transform {
                    // We need to convert our Vec2 into a Vec3, by giving it a z-coordinate
                    // This is used to determine the order of our sprites
                    translation: location.position().extend(0.0),
                    // The z-scale of 2D objects must always be 1.0,
                    // or their ordering will be affected in surprising ways.
                    // See https://github.com/bevyengine/bevy/issues/4149
                    scale: location.size().extend(1.0),
                    ..default()
                },
                sprite: Sprite {
                    color: WALL_COLOR,
                    ..default()
                },
                ..default()
            },
            collider: Collider,
        }
    }
}

// This resource tracks the game's score
#[derive(Resource)]
struct Scoreboard {
    score: usize,
}

// Add the game's entities to our world
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    asset_server: Res<AssetServer>,
) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    // Sound
    let ball_collision_sound = asset_server.load("sounds/breakout_collision.ogg");
    commands.insert_resource(CollisionSound(ball_collision_sound));

    // Paddle
    let paddle_y = -500.0;//BOTTOM_WALL + GAP_BETWEEN_PADDLE_AND_FLOOR;

    commands.spawn((
        SpriteBundle {
            transform: Transform {
                translation: Vec3::new(0.0, paddle_y, 0.0),
                scale: MARIO_SIZE,
                ..default()
            },
            sprite: Sprite {
                color: PACMAN_COLOR,
                ..default()
            },
            ..default()
        },
        Paddle,
        Collider,
    ));

    // Mario
    let texture: Handle<Image> = asset_server.load("mario.png");
    commands.spawn((
        /*MaterialMesh2dBundle {
            mesh: meshes.add(shape::Circle::default().into()).into(),
            material: materials.add(ColorMaterial::from(BALL_COLOR)),
            transform: Transform::from_translation(BALL_STARTING_POSITION).with_scale(BALL_SIZE),
            ..default()
        },*/
        SpriteBundle {
            transform: Transform::from_translation(MARIO_STARTING_POSITION).with_scale(MARIO_SIZE),
            texture: texture,
            sprite: Sprite{
                custom_size: Some(Vec2::new(1.0,1.0)),
                ..default()
            },
            ..default()
        },
        Mario,
        IsJumping{isjumping: false},
        Velocity(INITIAL_BALL_DIRECTION.normalize() * MARIO_XSPEED),
    ));

    // Scoreboard
    commands.spawn(
        TextBundle::from_sections([
            TextSection::new(
                "Score: ",
                TextStyle {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: SCOREBOARD_FONT_SIZE,
                    color: TEXT_COLOR,
                },
            ),
            TextSection::from_style(TextStyle {
                font: asset_server.load("fonts/FiraMono-Medium.ttf"),
                font_size: SCOREBOARD_FONT_SIZE,
                color: SCORE_COLOR,
            }),
        ])
        .with_style(Style {
            position_type: PositionType::Absolute,
            position: UiRect {
                top: SCOREBOARD_TEXT_PADDING,
                left: SCOREBOARD_TEXT_PADDING,
                ..default()
            },
            ..default()
        }),
    );

    // Walls
    //commands.spawn(WallBundle::new(WallLocation::Left));
    //commands.spawn(WallBundle::new(WallLocation::Right));
    commands.spawn(WallBundle::new(WallLocation::Bottom));
    //commands.spawn(WallBundle::new(WallLocation::Top));
    commands.spawn(WallBundle::new(WallLocation::Locate1));
    commands.spawn(WallBundle::new(WallLocation::Locate2));
    commands.spawn(WallBundle::new(WallLocation::Locate3));
    commands.spawn(WallBundle::new(WallLocation::Locate4));
    commands.spawn(WallBundle::new(WallLocation::Locate5));
    commands.spawn(WallBundle::new(WallLocation::Locate6));
    commands.spawn(WallBundle::new(WallLocation::Locate7));

    // Bricks
    // Negative scales result in flipped sprites / meshes,
    // which is definitely not what we want here
    assert!(BRICK_SIZE.x > 0.0);
    assert!(BRICK_SIZE.y > 0.0);

    let total_width_of_bricks = (RIGHT_WALL - LEFT_WALL) - 2. * GAP_BETWEEN_BRICKS_AND_SIDES;
    let bottom_edge_of_bricks = paddle_y + GAP_BETWEEN_PADDLE_AND_BRICKS;
    let total_height_of_bricks = TOP_WALL - bottom_edge_of_bricks - GAP_BETWEEN_BRICKS_AND_CEILING;

    assert!(total_width_of_bricks > 0.0);
    assert!(total_height_of_bricks > 0.0);

    // Given the space available, compute how many rows and columns of bricks we can fit
    let n_columns = (total_width_of_bricks / (BRICK_SIZE.x + GAP_BETWEEN_BRICKS)).floor() as usize;
    let n_rows = (total_height_of_bricks / (BRICK_SIZE.y + GAP_BETWEEN_BRICKS)).floor() as usize;
    let n_vertical_gaps = n_columns - 1;

    // Because we need to round the number of columns,
    // the space on the top and sides of the bricks only captures a lower bound, not an exact value
    let center_of_bricks = (LEFT_WALL + RIGHT_WALL) / 2.0;
    let left_edge_of_bricks = center_of_bricks
        // Space taken up by the bricks
        - (n_columns as f32 / 2.0 * BRICK_SIZE.x)
        // Space taken up by the gaps
        - n_vertical_gaps as f32 / 2.0 * GAP_BETWEEN_BRICKS;

    // In Bevy, the `translation` of an entity describes the center point,
    // not its bottom-left corner
    let offset_x = left_edge_of_bricks + BRICK_SIZE.x / 2.;
    let offset_y = bottom_edge_of_bricks + BRICK_SIZE.y / 2.;

    for row in 0..0 {
        for column in 0..0 {
            let brick_position = Vec2::new(
                offset_x + column as f32 * (BRICK_SIZE.x + GAP_BETWEEN_BRICKS),
                offset_y + row as f32 * (BRICK_SIZE.y + GAP_BETWEEN_BRICKS),
            );

            // brick
            commands.spawn((
                SpriteBundle {
                    sprite: Sprite {
                        color: BRICK_COLOR,
                        ..default()
                    },
                    transform: Transform {
                        translation: brick_position.extend(0.0),
                        scale: Vec3::new(BRICK_SIZE.x, BRICK_SIZE.y, 1.0),
                        ..default()
                    },
                    ..default()
                },
                Brick,
                Collider,
            ));
        }
    }
}

fn move_pacman(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<&mut Transform, With<Paddle>>,
) {
    let mut paddle_transform = query.single_mut();

    let x_direction = if keyboard_input.pressed(KeyCode::Left) {
        -1.0
    } else if keyboard_input.pressed(KeyCode::Right) {
        1.0
    } else {
        0.0
    };
    let y_direction = if keyboard_input.pressed(KeyCode::Down) {
        -1.0
    } else if keyboard_input.pressed(KeyCode::Up) {
        1.0
    } else {
        0.0
    };

    let new_paddle_x_position =
        paddle_transform.translation.x + x_direction * MARIO_XSPEED * TIME_STEP;
    let new_paddle_y_position =
        paddle_transform.translation.y + y_direction * MARIO_XSPEED * TIME_STEP;

    let left_bound = LEFT_WALL + WALL_THICKNESS / 2.0 + MARIO_SIZE.x / 2.0 + PADDLE_PADDING;
    let right_bound = RIGHT_WALL - WALL_THICKNESS / 2.0 - MARIO_SIZE.x / 2.0 - PADDLE_PADDING;
    let up_bound = TOP_WALL + WALL_THICKNESS / 2.0 + MARIO_SIZE.y / 2.0 + PADDLE_PADDING;
    let bottom_bound = BOTTOM_WALL - WALL_THICKNESS / 2.0 - MARIO_SIZE.y / 2.0 - PADDLE_PADDING;

    //paddle_transform.translation.x = new_paddle_x_position.clamp(left_bound, right_bound);
    //paddle_transform.translation.y = new_paddle_y_position.clamp(bottom_bound, up_bound);
}

fn move_mario_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut query: Query<(&mut Velocity, &mut Transform, &mut IsJumping), With<Mario>>,
) {
    let (mut ball_velocity, mut ball_transform, mut isjumping) = query.single_mut();
    if keyboard_input.pressed(KeyCode::Up) {
        if isjumping.isjumping == false{
            ball_velocity.y = JUMP_SPEED;
            isjumping.isjumping = true;
        }
        //ball_transform.rotation=Quat::from_rotation_z(-90.0_f32.to_radians());
    }
    
    /*if keyboard_input.pressed(KeyCode::Down) {
        ball_velocity.x = 0.0;
        ball_velocity.y = -BALL_SPEED;
        //ball_transform.rotation=Quat::from_rotation_z(90.0_f32.to_radians());
    }*/
    if keyboard_input.pressed(KeyCode::Left) {
        ball_velocity.x = -MARIO_XSPEED;
        //ball_transform.rotation=Quat::from_rotation_z(0.0_f32.to_radians());
    } else if keyboard_input.pressed(KeyCode::Right) {
        ball_velocity.x = MARIO_XSPEED;
        //ball_transform.rotation=Quat::from_rotation_z(180.0_f32.to_radians());
    } else {
        ball_velocity.x = 0.0;
    };
}

fn apply_velocity(mut query: Query<(&mut Transform, &mut Velocity, &IsJumping)>) {
    for (mut transform, mut velocity, isjumping) in &mut query {
        transform.translation.x += velocity.x * TIME_STEP;
        transform.translation.y += velocity.y * TIME_STEP;
        if transform.translation.x > BLOCK_SIZE * 16.0 {transform.translation.x = BLOCK_SIZE * -16.0}
        if transform.translation.x < BLOCK_SIZE * -16.0 {transform.translation.x = BLOCK_SIZE * 16.0}
        velocity.y -= GRAVITY;
    }
}

fn update_scoreboard(scoreboard: Res<Scoreboard>, mut query: Query<&mut Text>) {
    let mut text = query.single_mut();
    text.sections[1].value = scoreboard.score.to_string();
}

fn check_for_collisions(
    mut commands: Commands,
    mut scoreboard: ResMut<Scoreboard>,
    mut mario_query: Query<(&mut Velocity, &Transform, &mut IsJumping), With<Mario>>,
    collider_query: Query<(Entity, &Transform, Option<&Brick>), With<Collider>>,
    mut collision_events: EventWriter<CollisionEvent>,
) {
    let (mut mario_velocity, mario_transform, mut isjumping) = mario_query.single_mut();
    let ball_size = mario_transform.scale.truncate();

    // check collision with walls
    for (collider_entity, transform, maybe_brick) in &collider_query {
        let collision = collide(
            mario_transform.translation,
            ball_size,
            transform.translation,
            transform.scale.truncate(),
        );
        if let Some(collision) = collision {
            // Sends a collision event so that other systems can react to the collision
            collision_events.send_default();

            // Bricks should be despawned and increment the scoreboard on collision
            if maybe_brick.is_some() {
                scoreboard.score += 1;
                commands.entity(collider_entity).despawn();
            }else{

            // reflect the ball when it collides
            let mut reflect_x = false;
            let mut reflect_y = false;

            // only reflect if the ball's velocity is going in the opposite direction of the
            // collision
            match collision {
                Collision::Left => reflect_x = mario_velocity.x > 0.0,
                Collision::Right => reflect_x = mario_velocity.x < 0.0,
                Collision::Top => {reflect_y = mario_velocity.y < 0.0}
                Collision::Bottom => {if mario_velocity.y > 0.0 {mario_velocity.y = 0.0}}
                Collision::Inside => { /* do nothing */ }
            }

            // reflect velocity on the x-axis if we hit something on the x-axis
            if reflect_x {
                mario_velocity.x = 0.0;
            }

            // reflect velocity on the y-axis if we hit something on the y-axis
            if reflect_y {
                mario_velocity.y = 0.0;
                isjumping.isjumping = false;
            }
        }
        }
    }
}
