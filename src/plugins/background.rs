use bevy::{
    prelude::*,
    render::{camera::Camera, mesh::Indices, pipeline::PrimitiveTopology},
};
use bevy_prototype_lyon::prelude::*;
use log::{debug, error, info, trace, warn};

use super::genetics::SimulationParams;

pub struct BackgroundPlugin;

impl Plugin for BackgroundPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_startup_system(spawn_bg.system())
            .add_startup_system(setup_green_screen.system())
            .add_system(move_parallax.system());
    }
}

pub struct ParallaxComponent {
    initial_transform: Transform,
}

pub struct GreenScreenComponent;

fn spawn_bg(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    mut meshes: ResMut<Assets<Mesh>>,

    mut cmd: Commands,
) {
    let bg_tex = asset_server.load("textures/bg.png");

    let material = materials.add(ColorMaterial {
        texture: bg_tex.into(),
        ..Default::default()
    });

    let mesh = build_rect_uvs(Vec2::new(2., 2.), (-50., 50., -1., 2.));

    let sb = SpriteBundle {
        mesh: meshes.add(mesh),
        material,
        sprite: Sprite::new(Vec2::new(40000., 1080.)), //Note: sprite component is needed or scale is wrong
        transform: Transform::from_translation(Vec3::new(0., 0., -20.)),
        ..Default::default()
    };

    let initial_transform = sb.transform;
    cmd.spawn_bundle(sb)
        .insert(ParallaxComponent { initial_transform });
}

fn move_parallax(
    query_parallax: Query<(&mut Transform, &ParallaxComponent), Without<Camera>>,
    query_green_screen: Query<&mut Visible, With<GreenScreenComponent>>,
    query_cam: Query<&Transform, With<Camera>>,
    params: Res<SimulationParams>,
) {
    let cam_transform = query_cam.iter().next().unwrap();

    query_parallax.for_each_mut(|(mut bg_transform, bg_parallax)| {
        *bg_transform = *cam_transform;

        //Ignore Z
        bg_transform.translation.z = bg_parallax.initial_transform.translation.z;
    });

    query_green_screen.for_each_mut(|mut visible| {
        visible.is_visible = params.show_green_screen;
    });
}

//Source: https://github.com/JosePedroDias/rust_experiments/blob/main/bevy/src/shapes/rect.rs
pub fn build_rect_uvs(dims: Vec2, uvs: (f32, f32, f32, f32)) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);

    let n_vertices = 4;
    let n_indices = 6;

    let w2 = dims[0] / 2.0;
    let h2 = dims[1] / 2.0;

    let (u0, u1, v0, v1) = uvs;

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
    let mut normals: Vec<[f32; 3]> = Vec::with_capacity(n_vertices);
    let mut uvs: Vec<[f32; 2]> = Vec::with_capacity(n_vertices);

    // #0 tl
    positions.push([-w2, h2, 0.]);
    normals.push([0., 0., 1.]);
    uvs.push([u0, v0]);

    // #1 tr
    positions.push([w2, h2, 0.]);
    normals.push([0., 0., 1.]);
    uvs.push([u1, v0]);

    // #2 bl
    positions.push([-w2, -h2, 0.]);
    normals.push([0., 0., 1.]);
    uvs.push([u0, v1]);

    // #3 br
    positions.push([w2, -h2, 0.]);
    normals.push([0., 0., 1.]);
    uvs.push([u1, v1]);

    mesh.set_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.set_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.set_attribute(Mesh::ATTRIBUTE_UV_0, uvs);

    let mut indices: Vec<u32> = Vec::with_capacity(n_indices);
    indices.push(1);
    indices.push(0);
    indices.push(2);

    indices.push(1);
    indices.push(2);
    indices.push(3);
    mesh.set_indices(Some(Indices::U32(indices)));

    mesh
}

fn setup_green_screen(mut commands: Commands) {
    let mut builder = GeometryBuilder::new();
    builder.add(&shapes::Rectangle {
        width: 9000.,
        height: 9000.,
        ..Default::default()
    });

    let transform = Transform::from_translation(Vec3::new(0., 200., -10.));

    let shape_bundle = builder.build(
        ShapeColors::outlined(Color::GREEN, Color::GREEN),
        DrawMode::Fill(FillOptions::DEFAULT),
        transform,
    );

    commands
        .spawn_bundle(shape_bundle)
        .insert(ParallaxComponent {
            initial_transform: transform,
        })
        .insert(GreenScreenComponent)
        .insert(Name::new("Green Screen".to_owned()));
}
